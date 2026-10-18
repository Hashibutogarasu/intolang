extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavutil/imgutils.h>
#include <libswscale/swscale.h>
}

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

namespace {
const char *const RAMP = " .:-=+*#%@";
constexpr int RAMP_LENGTH = 10;
constexpr int BUFFER_SIZE = 32768;

struct FFmpeg {
    int placeholder = 0;
};

struct FsBytes {
    unsigned char *data;
    long long length;
};

struct BufferReader {
    const unsigned char *data;
    size_t length;
    size_t cursor;
};

int read_packet(void *opaque, uint8_t *buffer, int size) {
    auto *reader = static_cast<BufferReader *>(opaque);
    if (reader->cursor >= reader->length) return AVERROR_EOF;
    const size_t remaining = reader->length - reader->cursor;
    const size_t take = remaining < static_cast<size_t>(size) ? remaining : static_cast<size_t>(size);
    std::memcpy(buffer, reader->data + reader->cursor, take);
    reader->cursor += take;
    return static_cast<int>(take);
}

int64_t seek_packet(void *opaque, int64_t offset, int whence) {
    auto *reader = static_cast<BufferReader *>(opaque);
    if (whence & AVSEEK_SIZE) return static_cast<int64_t>(reader->length);
    whence &= ~AVSEEK_FORCE;
    int64_t base = 0;
    switch (whence) {
        case SEEK_SET: base = 0; break;
        case SEEK_CUR: base = static_cast<int64_t>(reader->cursor); break;
        case SEEK_END: base = static_cast<int64_t>(reader->length); break;
        default: return AVERROR(EINVAL);
    }
    const int64_t target = base + offset;
    if (target < 0) return AVERROR(EINVAL);
    reader->cursor = static_cast<size_t>(target);
    return target;
}

struct FFmpegVideo {
    AVFormatContext *format = nullptr;
    AVIOContext *avio = nullptr;
    BufferReader *reader = nullptr;
    FsBytes *owned = nullptr;
    AVCodecContext *decoder = nullptr;
    const AVCodec *codec = nullptr;
    AVStream *stream = nullptr;
    int stream_index = -1;
    SwsContext *scale = nullptr;
    AVFrame *frame = nullptr;
    AVPacket *packet = nullptr;
    int source_width = 0;
    int source_height = 0;
    int columns = 0;
    int rows = 0;
    int frame_delay_ms = 33;
    bool draining = false;
    std::vector<uint8_t> gray;
    std::string text;

    void build_text() {
        text.clear();
        text.reserve(static_cast<size_t>(rows) * static_cast<size_t>(columns + 1));
        for (int row = 0; row < rows; ++row) {
            for (int column = 0; column < columns; ++column) {
                const int luminance = gray[static_cast<size_t>(row) * columns + column];
                text.push_back(RAMP[luminance * (RAMP_LENGTH - 1) / 255]);
            }
            text.push_back('\n');
        }
    }
};

void release(FFmpegVideo *video) {
    if (!video) return;
    if (video->scale) sws_freeContext(video->scale);
    if (video->frame) av_frame_free(&video->frame);
    if (video->packet) av_packet_free(&video->packet);
    if (video->decoder) avcodec_free_context(&video->decoder);
    if (video->format) avformat_close_input(&video->format);
    if (video->avio) avio_context_free(&video->avio);
    delete video->reader;
    if (video->owned) {
        std::free(video->owned->data);
        std::free(video->owned);
        video->owned = nullptr;
    }
}

int scale_to_gray(FFmpegVideo *video) {
    if (!video->scale) return -1;
    uint8_t *destination[4] = {video->gray.data(), nullptr, nullptr, nullptr};
    int destination_linesize[4] = {video->columns, 0, 0, 0};
    return sws_scale(video->scale, video->frame->data, video->frame->linesize, 0, video->source_height,
                     destination, destination_linesize);
}

bool finish_video(FFmpegVideo *video, int32_t max_columns, int32_t max_rows) {
    if (avformat_find_stream_info(video->format, nullptr) < 0) return false;
    const int index = av_find_best_stream(video->format, AVMEDIA_TYPE_VIDEO, -1, -1, &video->codec, 0);
    if (index < 0 || !video->codec) return false;
    video->stream_index = index;
    video->stream = video->format->streams[index];
    video->decoder = avcodec_alloc_context3(video->codec);
    if (!video->decoder || avcodec_parameters_to_context(video->decoder, video->stream->codecpar) < 0 ||
        avcodec_open2(video->decoder, video->codec, nullptr) < 0) {
        return false;
    }
    video->source_width = video->decoder->width;
    video->source_height = video->decoder->height;
    if (video->source_width <= 0 || video->source_height <= 0) return false;
    if (video->decoder->pix_fmt == AV_PIX_FMT_NONE) return false;

    const double aspect = static_cast<double>(video->source_width) / static_cast<double>(video->source_height);
    int columns = max_columns > 0 ? max_columns : 80;
    int rows = static_cast<int>(std::lround(static_cast<double>(columns) / aspect));
    if (rows < 1) rows = 1;
    if (max_rows > 0 && rows > max_rows) {
        rows = max_rows;
        columns = static_cast<int>(std::lround(static_cast<double>(rows) * aspect));
        if (columns < 1) columns = 1;
        if (max_columns > 0 && columns > max_columns) columns = max_columns;
    }
    video->columns = columns;
    video->rows = rows;
    video->gray.resize(static_cast<size_t>(columns) * static_cast<size_t>(rows));
    video->scale = sws_getContext(video->source_width, video->source_height, video->decoder->pix_fmt,
                                  columns, rows, AV_PIX_FMT_GRAY8, SWS_BILINEAR, nullptr, nullptr, nullptr);
    video->frame = av_frame_alloc();
    video->packet = av_packet_alloc();
    if (!video->scale || !video->frame || !video->packet) return false;

    AVRational frame_rate = video->stream->avg_frame_rate;
    if (frame_rate.num == 0 || frame_rate.den == 0) frame_rate = video->stream->r_frame_rate;
    const double fps = frame_rate.den > 0 ? static_cast<double>(frame_rate.num) / static_cast<double>(frame_rate.den) : 0.0;
    video->frame_delay_ms = fps > 0.5 ? static_cast<int>(std::lround(1000.0 / fps)) : 33;
    if (video->frame_delay_ms < 1) video->frame_delay_ms = 1;
    return true;
}
}

extern "C" FFmpeg *ffmpeg_new() {
    return new FFmpeg();
}

extern "C" FFmpegVideo *ffmpeg_video_open(const char *path, int32_t max_columns, int32_t max_rows) {
    if (!path) return nullptr;
    auto *video = new FFmpegVideo();
    if (avformat_open_input(&video->format, path, nullptr, nullptr) < 0 ||
        !finish_video(video, max_columns, max_rows)) {
        release(video);
        delete video;
        return nullptr;
    }
    return video;
}

extern "C" FFmpegVideo *ffmpeg_open_buffer(FFmpeg *ffmpeg, FsBytes *bytes) {
    (void)ffmpeg;
    if (!bytes) return nullptr;
    if (!bytes->data || bytes->length <= 0) {
        std::free(bytes->data);
        std::free(bytes);
        return nullptr;
    }
    auto *video = new FFmpegVideo();
    video->owned = bytes;
    video->reader = new BufferReader{bytes->data, static_cast<size_t>(bytes->length), 0};
    unsigned char *buffer = static_cast<unsigned char *>(av_malloc(BUFFER_SIZE));
    if (!buffer) {
        release(video);
        delete video;
        return nullptr;
    }
    video->avio = avio_alloc_context(buffer, BUFFER_SIZE, 0, video->reader, read_packet, nullptr, seek_packet);
    if (!video->avio) {
        av_free(buffer);
        release(video);
        delete video;
        return nullptr;
    }
    video->avio->seekable = AVIO_SEEKABLE_NORMAL;
    video->format = avformat_alloc_context();
    if (!video->format) {
        release(video);
        delete video;
        return nullptr;
    }
    video->format->pb = video->avio;
    if (avformat_open_input(&video->format, nullptr, nullptr, nullptr) < 0 ||
        !finish_video(video, 106, 60)) {
        release(video);
        delete video;
        return nullptr;
    }
    return video;
}

extern "C" FFmpegVideo *ffmpeg_read(FFmpeg *ffmpeg, const char *path) {
    (void)ffmpeg;
    return ffmpeg_video_open(path, 106, 60);
}

extern "C" int32_t ffmpeg_video_columns(FFmpegVideo *video) {
    return video ? video->columns : 0;
}

extern "C" int32_t ffmpeg_video_rows(FFmpegVideo *video) {
    return video ? video->rows : 0;
}

extern "C" int32_t ffmpeg_video_frame_delay_ms(FFmpegVideo *video) {
    return video ? video->frame_delay_ms : 0;
}

extern "C" bool ffmpeg_video_read_frame(FFmpegVideo *video) {
    if (!video || !video->decoder) return false;
    for (;;) {
        const int received = avcodec_receive_frame(video->decoder, video->frame);
        if (received == 0) {
            if (scale_to_gray(video) <= 0) {
                av_frame_unref(video->frame);
                return false;
            }
            video->build_text();
            av_frame_unref(video->frame);
            return true;
        }
        if (received == AVERROR_EOF) return false;
        if (received != AVERROR(EAGAIN)) return false;
        if (video->draining) return false;
        const int packet = av_read_frame(video->format, video->packet);
        if (packet < 0) {
            avcodec_send_packet(video->decoder, nullptr);
            video->draining = true;
            continue;
        }
        if (video->packet->stream_index == video->stream_index) {
            avcodec_send_packet(video->decoder, video->packet);
        }
        av_packet_unref(video->packet);
    }
}

extern "C" const char *ffmpeg_video_text(FFmpegVideo *video) {
    return video ? video->text.c_str() : "";
}

extern "C" void ffmpeg_video_destroy(FFmpegVideo *video) {
    if (!video) return;
    release(video);
    delete video;
}

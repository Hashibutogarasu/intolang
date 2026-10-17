extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavutil/channel_layout.h>
#include <libavutil/mathematics.h>
#include <libavutil/samplefmt.h>
#include <libswresample/swresample.h>
}

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <vector>

namespace {
constexpr int BUFFER_SIZE = 32768;

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

struct Media {
    AVFormatContext *format = nullptr;
    AVIOContext *avio = nullptr;
    BufferReader *reader = nullptr;
    AVCodecContext *decoder = nullptr;
    SwrContext *resampler = nullptr;
    AVPacket *packet = nullptr;
    AVFrame *frame = nullptr;
    int stream_index = -1;
    int sample_rate = 0;
    int channels = 0;
    std::vector<int16_t> buffer;
    uint8_t *planes[1] = {nullptr};
};

void release(Media *media) {
    if (media->format) avformat_close_input(&media->format);
    if (media->avio) avio_context_free(&media->avio);
    if (media->decoder) avcodec_free_context(&media->decoder);
    if (media->resampler) swr_free(&media->resampler);
    if (media->packet) av_packet_free(&media->packet);
    if (media->frame) av_frame_free(&media->frame);
    delete media->reader;
}

Media *open_media(const char *path, const unsigned char *data, size_t length) {
    auto *media = new Media();
    if (path) {
        if (avformat_open_input(&media->format, path, nullptr, nullptr) < 0) goto fail;
    } else if (data && length > 0) {
        media->reader = new BufferReader{data, length, 0};
        unsigned char *buffer = static_cast<unsigned char *>(av_malloc(BUFFER_SIZE));
        if (!buffer) goto fail;
        media->avio = avio_alloc_context(buffer, BUFFER_SIZE, 0, media->reader, read_packet,
                                         nullptr, seek_packet);
        if (!media->avio) {
            av_free(buffer);
            goto fail;
        }
        media->avio->seekable = AVIO_SEEKABLE_NORMAL;
        media->format = avformat_alloc_context();
        if (!media->format) goto fail;
        media->format->pb = media->avio;
        if (avformat_open_input(&media->format, nullptr, nullptr, nullptr) < 0) goto fail;
    } else {
        goto fail;
    }

    if (avformat_find_stream_info(media->format, nullptr) < 0) goto fail;
    {
        const AVCodec *codec = nullptr;
        const int index = av_find_best_stream(media->format, AVMEDIA_TYPE_AUDIO, -1, -1, &codec, 0);
        if (index < 0 || !codec) goto fail;
        media->stream_index = index;
        media->decoder = avcodec_alloc_context3(codec);
        if (!media->decoder ||
            avcodec_parameters_to_context(media->decoder, media->format->streams[index]->codecpar) < 0 ||
            avcodec_open2(media->decoder, codec, nullptr) < 0) {
            goto fail;
        }
        media->sample_rate = media->decoder->sample_rate;

        AVChannelLayout in_layout;
        if (media->decoder->ch_layout.nb_channels > 0) {
            av_channel_layout_copy(&in_layout, &media->decoder->ch_layout);
        } else {
            av_channel_layout_default(&in_layout,
                                      media->decoder->channels > 0 ? media->decoder->channels : 2);
        }
        AVChannelLayout out_layout;
        av_channel_layout_default(&out_layout, in_layout.nb_channels);
        const int configured =
            swr_alloc_set_opts2(&media->resampler, &out_layout, AV_SAMPLE_FMT_S16, media->sample_rate,
                                &in_layout, media->decoder->sample_fmt, media->sample_rate, 0, nullptr);
        media->channels = in_layout.nb_channels;
        av_channel_layout_uninit(&in_layout);
        av_channel_layout_uninit(&out_layout);
        if (configured < 0 || !media->resampler || swr_init(media->resampler) < 0) goto fail;

        media->packet = av_packet_alloc();
        media->frame = av_frame_alloc();
        if (!media->packet || !media->frame) goto fail;
    }
    return media;

fail:
    release(media);
    delete media;
    return nullptr;
}
}

extern "C" void *ffmpeg_media_open_path(const char *path) {
    if (!path) return nullptr;
    return open_media(path, nullptr, 0);
}

extern "C" void *ffmpeg_media_open_buffer(FsBytes *bytes) {
    if (!bytes || !bytes->data || bytes->length <= 0) return nullptr;
    return open_media(nullptr, bytes->data, static_cast<size_t>(bytes->length));
}

extern "C" void ffmpeg_media_destroy(void *handle) {
    auto *media = static_cast<Media *>(handle);
    if (!media) return;
    release(media);
    delete media;
}

extern "C" void *ffmpeg_media_format(void *handle) {
    return handle ? static_cast<Media *>(handle)->format : nullptr;
}

extern "C" void *ffmpeg_media_decoder(void *handle) {
    return handle ? static_cast<Media *>(handle)->decoder : nullptr;
}

extern "C" void *ffmpeg_media_packet(void *handle) {
    return handle ? static_cast<Media *>(handle)->packet : nullptr;
}

extern "C" void *ffmpeg_media_frame(void *handle) {
    return handle ? static_cast<Media *>(handle)->frame : nullptr;
}

extern "C" void *ffmpeg_media_resampler(void *handle) {
    return handle ? static_cast<Media *>(handle)->resampler : nullptr;
}

extern "C" int32_t ffmpeg_media_stream_index(void *handle) {
    return handle ? static_cast<Media *>(handle)->stream_index : -1;
}

extern "C" int32_t ffmpeg_media_sample_rate(void *handle) {
    return handle ? static_cast<Media *>(handle)->sample_rate : 0;
}

extern "C" int32_t ffmpeg_media_channels(void *handle) {
    return handle ? static_cast<Media *>(handle)->channels : 0;
}

extern "C" int32_t ffmpeg_media_out_capacity(void *handle) {
    auto *media = static_cast<Media *>(handle);
    if (!media || !media->frame) return 0;
    const int64_t delay = swr_get_delay(media->resampler, media->sample_rate);
    return static_cast<int32_t>(av_rescale_rnd(delay + media->frame->nb_samples, media->sample_rate,
                                               media->sample_rate, AV_ROUND_UP));
}

extern "C" void *ffmpeg_media_reserve(void *handle, int32_t samples) {
    auto *media = static_cast<Media *>(handle);
    if (!media || samples <= 0 || media->channels <= 0) return nullptr;
    const size_t count = static_cast<size_t>(samples) * static_cast<size_t>(media->channels);
    if (media->buffer.size() < count) media->buffer.resize(count);
    media->planes[0] = reinterpret_cast<uint8_t *>(media->buffer.data());
    return media->planes;
}

extern "C" void *ffmpeg_media_buffer(void *handle) {
    auto *media = static_cast<Media *>(handle);
    return media ? media->planes[0] : nullptr;
}

extern "C" int32_t ffmpeg_packet_stream(void *packet) {
    return packet ? static_cast<AVPacket *>(packet)->stream_index : -1;
}

extern "C" int32_t ffmpeg_frame_samples(void *frame) {
    return frame ? static_cast<AVFrame *>(frame)->nb_samples : 0;
}

extern "C" void *ffmpeg_frame_data(void *frame) {
    return frame ? static_cast<AVFrame *>(frame)->extended_data : nullptr;
}

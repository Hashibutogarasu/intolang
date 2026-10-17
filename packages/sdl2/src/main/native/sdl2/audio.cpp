#include <SDL2/SDL.h>

#include <cstdint>

namespace {
SDL_AudioDeviceID device = 0;
}

extern "C" void *sdl2_new() {
    return &device;
}

extern "C" void *sdl2_open(void *self, int32_t frequency, int32_t channels) {
    (void)self;
    if (SDL_InitSubSystem(SDL_INIT_AUDIO) != 0) return nullptr;
    SDL_AudioSpec want;
    SDL_zero(want);
    want.freq = frequency;
    want.format = AUDIO_S16SYS;
    want.channels = static_cast<Uint8>(channels);
    want.samples = 4096;
    device = SDL_OpenAudioDevice(nullptr, 0, &want, nullptr, 0);
    if (device == 0) return nullptr;
    return &device;
}

extern "C" void sdl2_queue(void *self, const void *samples, int32_t bytes) {
    (void)self;
    if (device == 0 || !samples || bytes <= 0) return;
    SDL_QueueAudio(device, samples, static_cast<Uint32>(bytes));
}

extern "C" void sdl2_pause(void *self) {
    (void)self;
    if (device != 0) SDL_PauseAudioDevice(device, 1);
}

extern "C" void sdl2_resume(void *self) {
    (void)self;
    if (device != 0) SDL_PauseAudioDevice(device, 0);
}

extern "C" void sdl2_clear(void *self) {
    (void)self;
    if (device != 0) SDL_ClearQueuedAudio(device);
}

extern "C" int32_t sdl2_queued(void *self) {
    (void)self;
    return device == 0 ? 0 : static_cast<int32_t>(SDL_GetQueuedAudioSize(device));
}

extern "C" void sdl2_close(void *self) {
    (void)self;
    if (device != 0) {
        SDL_CloseAudioDevice(device);
        device = 0;
    }
}

extern "C" void sdl2_destroy(void *self) {
    (void)self;
    sdl2_close(self);
    SDL_QuitSubSystem(SDL_INIT_AUDIO);
}

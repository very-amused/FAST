#pragma once
#include <stdint.h>
#include <stdbool.h>

// A fast STREAM instance which consumes audio frames on a clock,
// simulating an audio stream connected to a real audio server such as pulseaudio or pipewire
//
// This is the terminal point of audio for fast: read into a black hole.
typedef struct FastStream FastStream;

// Settings for creating a FastStream
typedef struct FastStreamSettings {
	uint8_t sample_size; // byte size of one audio sample (e.g 2 for 16-bit samples)
	uint32_t n_channels; // Number of audio channels to simulate
	uint32_t sample_rate; // Sample rate, i.e 44100 for 44.1khz

	uint32_t buffer_ms; // ms of audio to buffer
} FastStreamSettings;

// Allocate and initialize a new FastStream
// NOTE: The stream starts in a paused/corked state and is not started until [FastStream_start] is called.
//
// Returns NULL on error.
FastStream *FastStream_new(const FastStreamSettings *settings);
// Stop, deinitialize and free a FastStream
void FastStream_free(FastStream *stream);

// Start a FastStream, causing it to start reading audio from its buffer and requesting audio frames
//
// Returns 0 on success, nonzero on error
int FastStream_start(FastStream *stream);

// Play/pause a FastStream, blocking until the desired play/pause state is achieved
int FastStream_play(FastStream *stream, bool play);

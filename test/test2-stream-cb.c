
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "stream.h"

static const FastStreamSettings STREAM_SETTINGS = {
	.sample_size = 2,
	.n_channels = 1,
	.sample_rate = 44100,
	.buffer_ms = 250
};

static const size_t WAV_LEN = 5 * (STREAM_SETTINGS.sample_rate * STREAM_SETTINGS.n_channels * STREAM_SETTINGS.sample_size);

static void stream_write_cb(FastStream *stream, size_t n_bytes, void *userdata);

int main() {
	// Allocate a buffer that mimics audio data for our wav (5 sec)
	unsigned char *wav = malloc(WAV_LEN);
	memset(wav, 0x00, WAV_LEN);

	// Create stream
	FastStream *stream = FastStream_new(&STREAM_SETTINGS);
	if (!stream) {
		fprintf(stderr, "Failed to create stream\n");
		return 1;
	}

	// Set write callback
	FastStream_set_write_cb(stream, stream_write_cb, wav);

	// Start stream
	if (FastStream_start(stream) != 0) {
		fprintf(stderr, "Error calling FastStream_start\n");
	}

	for (int i = 0; i < 3; i++) {
		// Play for 2s (200 ticks)
		sleep(2);

		// Pause for 1s
		if (FastStream_play(stream, false) != 0) {
			fprintf(stderr, "Error pausing with FastStream_play\n");
		}
		sleep(1);

		// Play for 1s (100 ticks)
		if (FastStream_play(stream, true) != 0) {
			fprintf(stderr, "Error playing with FastStream_play\n");
		}
		sleep(1);

		// Pause for 1s
		if (FastStream_play(stream, false) != 0) {
			fprintf(stderr, "Error pausing with FastStream_play\n");
		}
		sleep(1);
		if (i == 2) {
			break;
		} else if (FastStream_play(stream, true) != 0) {
			fprintf(stderr, "Error playing with FastStream_play\n");
		}
	}

	// Cleanup
	FastStream_free(stream);

	return 0;
}

static void stream_write_cb(FastStream *stream, size_t n_bytes, void *userdata) {
	static size_t n = 0;
	unsigned char *wavdata = userdata;

	int status = FastStream_write(stream, &wavdata[n], n_bytes);
	if (status == 0) {
		n += n_bytes;
	} else {
		fprintf(stderr, "FastStream_write failed in stream_write_cb\n");
	}
}

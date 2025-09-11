#include <stdbool.h>
#include <stdio.h>
#include <unistd.h>

#include "stream.h"

static const FastStreamSettings STREAM_SETTINGS = {
	.sample_size = 2,
	.n_channels = 1,
	.sample_rate = 44100,
	.buffer_ms = 250
};

int main() {
	// Create stream
	FastStream *stream = FastStream_new(&STREAM_SETTINGS);
	if (!stream) {
		fprintf(stderr, "Failed to create stream\n");
		return 1;
	}

	// Start stream and play for 2s
	if (FastStream_start(stream) != 0) {
		fprintf(stderr, "Error calling FastStream_start\n");
	}

	sleep(2);
	if (FastStream_play(stream, false) != 0) {
		fprintf(stderr, "Error pausing with FastStream_play\n");
	}


	// Cleanup
	FastStream_free(stream);

	return 0;
}

#include <stdbool.h>
#include <stdio.h>
#include <unistd.h>

#include "server.h"
#include "stream.h"

static const FastStreamSettings STREAM_SETTINGS = {
	.sample_size = 2,
	.n_channels = 1,
	.sample_rate = 44100,
	.buffer_ms = 250
};

int main() {
	// Create server
	FastServer *srv = FastServer_new();
	if (!srv) {
		fprintf(stderr, "Failed to create server\n");
		return 1;
	}

	// Create stream
	FastStream *stream = FastStream_new(srv, &STREAM_SETTINGS);
	if (!stream) {
		fprintf(stderr, "Failed to create stream\n");
		return 1;
	}

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
	FastServer_free(srv);

	return 0;
}

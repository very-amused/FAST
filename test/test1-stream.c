#include <stdbool.h>
#include <stdio.h>
#include <unistd.h>

#include "loop.h"
#include "server.h"
#include "stream.h"

static const FastStreamSettings STREAM_SETTINGS = {
	.sample_size = 2,
	.n_channels = 1,
	.sample_rate = 44100,
	.buffer_ms = 250
};

int main() {
	// Create server and loop
	FastServer *srv = FastServer_new();
	if (!srv) {
		fprintf(stderr, "Failed to create server\n");
		return 1;
	}

	FastLoop *loop = FastLoop_new(srv);
	if (!loop) {
		fprintf(stderr, "Failed to create event loop\n");
		return 1;
	}

	// Create stream
	FastStream *stream = FastStream_new(loop, &STREAM_SETTINGS);
	if (!stream) {
		fprintf(stderr, "Failed to create stream\n");
		return 1;
	}

	// Start stream
	if (FastStream_play(stream, true) != 0) {
		fprintf(stderr, "Error calling FastStream_start\n");
	}

	static int n_loops = 1;
	for (int i = 0; i < n_loops; i++) {
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

		// Play for 1s
		if (FastStream_play(stream, true) != 0) {
			fprintf(stderr, "Error playing with FastStream_play\n");
		}
		sleep(1);

		if (i == n_loops-1) {
			break;
		}
		if (FastStream_play(stream, true) != 0) {
			fprintf(stderr, "Error playing with FastStream_play\n");
		}
	}



	// Cleanup
	FastStream_free(stream);
	FastLoop_free(loop);
	FastServer_free(srv);

	return 0;
}

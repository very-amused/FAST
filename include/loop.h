#pragma once
#include <stdint.h>

// An asynchronous event loop analogue that provides a lock needed to interact with a FAST server.
// Callbacks are automatically run with the lock acquired. Thus, acquiring a FastLoop lock outside a callback
// will prevent callbacks from running while the lock is held.
typedef struct FastLoop FastLoop;

// Allocate and initialize a new FastLoop. Since this is only an emulation
// of a real asynchronous event loop, there is no "starting"/"stopping" of the loop.
// Once allocated, the loop is ready for use.
//
// Returns NULL on error.
FastLoop *FastLoop_new();
// Deinitialize and free a FastLoop.
void FastLoop_free(FastLoop *loop);

// Lock a FastLoop, preventing other threads from interacting with FAST
// while the lock is held.
void FastLoop_lock(FastLoop *loop);
// Unlock a FastLoop locked via [FastLoop_lock]
void FastLoop_unlock(FastLoop *loop);

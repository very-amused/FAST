#pragma once

// A FAST instance which embeds a runtime needed to schedule things like audio sink streams (see FastStream) and callbacks (see FastLoop).
// A FastServer MUST be the first thing you create and the last you destroy when using FAST.
typedef struct FastServer FastServer;

// Allocate and initialize a new FastServer. Under the hood, this embeds a Tokio runtime that powers FAST's functionality. 
//
// Returns NULL on error.
FastServer *FastServer_new();
// Deinitialize a free a FastServer.
//
// Returns 0 on success, nonzero on error.
int FastServer_free(FastServer *srv);

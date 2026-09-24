/* Compile the unchanged vendored parser after system headers. macOS exposes
 * sig_t through its headers, while parser v0.3.0 uses that name privately.
 * Scope a private identifier rename to the parser body only. */
#include "moenotes_chart_parser.h"
#include "yyjson.h"
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <zlib.h>
#define sig_t moenotes_private_signature_t
#include "../../vendor/moenotes-chart-parser/src/moenotes_chart_parser.c"
#undef sig_t

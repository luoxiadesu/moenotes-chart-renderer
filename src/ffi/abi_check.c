#include "moenotes_chart_parser.h"
#include <stddef.h>
#ifdef __EMSCRIPTEN__
_Static_assert(sizeof(void*) == 4, "wasm32 ABI required");
_Static_assert(sizeof(moenotes_allocator_t) == 16, "wasm allocator ABI changed");
_Static_assert(offsetof(moenotes_allocator_t, free_fn) == 12, "wasm allocator offset changed");
#else
_Static_assert(sizeof(void*) == 8, "64-bit native ABI required");
_Static_assert(sizeof(moenotes_allocator_t) == 32, "native allocator ABI changed");
#endif
_Static_assert(sizeof(moenotes_parse_options_t) == 12, "options ABI changed");
_Static_assert(sizeof(moenotes_position_t) == 32, "position ABI changed");
_Static_assert(sizeof(moenotes_note_view_t) == 136, "note ABI changed");
_Static_assert(offsetof(moenotes_note_view_t, position) == 16, "position offset changed");
_Static_assert(sizeof(moenotes_event_t) == 88, "event ABI changed");
_Static_assert(sizeof(moenotes_line_sample_t) == 24, "sample ABI changed");
_Static_assert(sizeof(moenotes_line_view_t) == 24, "line ABI changed");
_Static_assert(sizeof(moenotes_bpm_event_t) == 48, "BPM ABI changed");
_Static_assert(sizeof(moenotes_signature_event_t) == 48, "signature ABI changed");

#pragma once

#include <purple.h>

#if !(GLIB_CHECK_VERSION(2, 67, 3))
#define g_memdup2 g_memdup
#endif

#if PURPLE_VERSION_CHECK(3, 0, 0)
#include "purple-3.h"
#else
#include "purple-2.h"
#endif

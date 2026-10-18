// Load-time entry of the end-to-end driver. The driver library is never part of the app bundle:
// the test runner injects it into the release app with DYLD_INSERT_LIBRARIES (apps/macos/E2E).
#include <stdlib.h>
extern void openframe_e2e_start(void);
__attribute__((constructor)) static void openframe_e2e_load(void) {
    if (getenv("OPENFRAME_E2E_SCENARIO") != NULL) openframe_e2e_start();
}

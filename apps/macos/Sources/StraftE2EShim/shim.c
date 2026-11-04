// Load-time entry of the end-to-end driver. The driver library is never part of the app bundle:
// the test runner injects it into the release app with DYLD_INSERT_LIBRARIES (apps/macos/E2E).
#include <stdlib.h>
extern void straft_e2e_start(void);
__attribute__((constructor)) static void straft_e2e_load(void) {
    if (getenv("STRAFT_E2E_SCENARIO") != NULL) straft_e2e_start();
}

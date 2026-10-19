import "@testing-library/jest-dom/vitest";
import { configure } from "@testing-library/react";

// `findBy*` and `waitFor` give up after 1 s by default. Whole workspace tabs (React Query plus
// lazy panels) can take longer on a loaded build machine, which made assertions fail at random
// (visual.test.tsx failed 3 runs in 5 on 2026-10-05). A longer ceiling only waits when the UI
// is slow; a real failure still fails, within the 15 s test timeout.
configure({ asyncUtilTimeout: 5_000 });

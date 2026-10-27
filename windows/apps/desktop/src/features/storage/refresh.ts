// Settings > Storage reads storage.info once per open; a change of place asks for it again.

import { queryClient } from "../../ipc/query";

export const refreshStorage = () => void queryClient.invalidateQueries({ queryKey: ["storage.info"] });

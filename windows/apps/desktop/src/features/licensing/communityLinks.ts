// The community accounts shown on the welcome popup. These eight addresses are the only ones the
// popup opens; each is a fixed https address (owner decision 2026-11-05). The Discord entry is the
// website's own forwarding page, so the app never holds an invite code that can expire.
// Nothing here is fetched: an address is used only when the person clicks it.

export const COMMUNITY_LINKS = [
  { name: "Instagram", url: "https://www.instagram.com/straftstudio/" },
  { name: "YouTube", url: "https://www.youtube.com/@StraftStudio" },
  { name: "X", url: "https://x.com/straftstudio/" },
  { name: "Facebook", url: "https://www.facebook.com/straftstudio/" },
  { name: "LinkedIn", url: "https://www.linkedin.com/company/straftstudio/" },
  { name: "Reddit", url: "https://www.reddit.com/user/straftstudio/" },
  { name: "Discord", url: "https://straftstudio.com/discord" },
  { name: "Threads", url: "https://www.threads.com/@straftstudio/" },
] as const;

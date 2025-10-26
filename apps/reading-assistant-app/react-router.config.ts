import type { Config } from "@react-router/dev/config";

// This configuration is for the React Router dev tools.
// We're specifying a client-side only app (no SSR).
export default {
  ssr: false,
} satisfies Config;

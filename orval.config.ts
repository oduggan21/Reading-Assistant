import type { Config } from "@orval/core";

export default {
  api: {
    output: {
      // Use 'tags-split' mode. Orval will create a file for each tag
      // found in the OpenAPI spec. We will ensure our spec has one tag.
      mode: "tags-split",
      // The target directory for the generated files.
      target: "./packages/reading-assistant-query/src",
      client: "react-query",
      mock: false,
      override: {
        mutator: {
          path: './packages/reading-assistant-query/src/axios.ts',
          name: 'customInstance',
        },
      },
    },
    input: {
      target: "./openapi.json",
    },
  },
} satisfies Config;


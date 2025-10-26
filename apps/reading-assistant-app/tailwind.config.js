/** @type {import('tailwindcss').Config} */
export default {
  content: [
    // Scan all the files inside the `app` directory
    "./app/**/*.{js,ts,jsx,tsx}",
    // Also scan the shared UI package for components
    "../../packages/ui/src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}

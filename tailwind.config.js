// tailwind.config.js
export default {
    darkMode: 'class',
    content: [
      "./index.html",
      "./src/**/*.{js,ts,jsx,tsx}",
    ],
    theme: {
      extend: {
        colors: {
          light: {
            bg: "#ffffff",
            card: "#f9f9f9",
            text: "#222222",
            accent: "#ff5ca8",
            border: "#e5e7eb",
            input: "#ffffff",
            highlight: "#ffe3f1",
          },
          dark: {
            bg: "#0d0c1b",
            card: "#1a152b",
            text: "#f4f4ff",
            accent: "#c084fc",
            border: "#2e2b40",
            input: "#1c1b29",
            highlight: "#3a2e4f",
          },
        },
      },
    },
    plugins: [],
  };
  
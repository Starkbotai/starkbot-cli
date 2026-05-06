/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          0: "#0a0a0f",
          1: "#12121a",
          2: "#1a1a25",
          3: "#222230",
        },
        accent: {
          DEFAULT: "#6366f1",
          light: "#818cf8",
          dim: "#4f46e5",
        },
      },
    },
  },
  plugins: [],
};

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: {
          950: "#060b18",
          900: "#0a1224",
          800: "#0f1a32",
          700: "#162445",
          600: "#1e3058",
          500: "#2a4270",
        },
        fog: {
          100: "#e8eef9",
          200: "#c5d0e6",
          300: "#9aabc9",
          500: "#6b7d9e",
        },
        signal: { DEFAULT: "#4d8dff", dim: "#3a6fd4", glow: "#7eb0ff" },
        cyan: { DEFAULT: "#22d3ee", dim: "#0891b2" },
        purple: { DEFAULT: "#a78bfa", dim: "#7c5cbf" },
        ok: "#34d399",
        warn: "#fbbf24",
        bad: "#f87171",
      },
      fontFamily: {
        sans: ['"Segoe UI Variable"', '"Segoe UI"', "system-ui", "-apple-system", "Roboto", "Ubuntu", "Cantarell", "sans-serif"],
      },
      boxShadow: {
        card: "0 0 0 1px rgba(77,141,255,0.12), 0 8px 32px rgba(0,0,0,0.35)",
        glow: "0 0 24px rgba(77,141,255,0.25)",
        "glow-green": "0 0 20px rgba(52,211,153,0.2)",
        "glow-purple": "0 0 20px rgba(167,139,250,0.2)",
      },
      keyframes: {
        fadeUp: {
          "0%": { opacity: "0", transform: "translateY(8px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        pulseSoft: {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.55" },
        },
        shimmer: {
          "0%": { backgroundPosition: "0% 50%" },
          "100%": { backgroundPosition: "100% 50%" },
        },
      },
      animation: {
        "fade-up": "fadeUp 0.35s ease-out both",
        "pulse-soft": "pulseSoft 2s ease-in-out infinite",
      },
    },
  },
  plugins: [],
};

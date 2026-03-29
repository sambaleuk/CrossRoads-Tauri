/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        neon: {
          green: '#0DF170',
          blue: '#00D4FF',
          purple: '#8B5CF6',
        },
      },
    },
  },
  plugins: [],
};

import type { Config } from 'tailwindcss'

const config: Config = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        bmux: {
          bg: '#0d1117',
          surface: '#161b22',
          border: '#30363d',
          accent: '#58a6ff',
          green: '#3fb950',
          yellow: '#d29922',
          red: '#f85149',
          purple: '#bc8cff',
        },
      },
    },
  },
  plugins: [],
}

export default config

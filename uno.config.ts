import { defineConfig, presetUno, presetIcons, presetTypography } from 'unocss';

export default defineConfig({
  presets: [
    presetUno({
      dark: 'class', // Enable manual dark mode via class toggle
    }),
    presetIcons(),
    presetTypography(),
  ],
});

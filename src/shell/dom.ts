// Mounts the application shell (desktop + taskbar) and exposes its elements.
// This module has no dependency on other app modules, so it is safe to run at import time.
import iconTerminal from '../assets/icons/terminal-2.svg?raw';
import iconFolder from '../assets/icons/folder.svg?raw';
import iconCategory from '../assets/icons/category.svg?raw';
import iconVolume from '../assets/icons/volume.svg?raw';
import iconVolume4 from '../assets/icons/volume-4.svg?raw';
import { TASKBAR_HEIGHT_PX, TITLE_BAR_HEIGHT } from '../constants';

const root = document.querySelector<HTMLDivElement>('#app');

if (!root) {
  throw new Error('Root element #app was not found.');
}

root.innerHTML = `
  <main class="app-shell">
    <section id="desktop" class="desktop" aria-label="Desktop"></section>
    <footer class="taskbar" aria-label="Taskbar">
      <div class="taskbar-left">
        <button id="hamburger-menu" type="button" class="hamburger-menu" aria-label="Menu"></button>
        <div id="menu-popup" class="menu-popup hidden" role="menu">
          <button id="menu-new-terminal" type="button" class="menu-item" role="menuitem">ターミナル</button>
          <button id="menu-file" type="button" class="menu-item" role="menuitem">ファイル</button>
          <button id="menu-about" type="button" class="menu-item" role="menuitem">About...</button>
        </div>
      </div>
      <div id="taskbar-center" class="taskbar-center" aria-label="Window buttons"></div>
      <div class="taskbar-actions">
        <div class="taskbar-volume-container">
          <button id="taskbar-volume-button" type="button" class="taskbar-volume-button" aria-label="Volume"></button>
          <div id="volume-popup" class="volume-popup hidden">
            <div class="volume-display" id="volume-display">50</div>
            <div class="volume-controls">
              <span class="volume-icon volume-icon-min">${iconVolume4}</span>
              <input id="volume-slider" type="range" class="volume-slider" min="0" max="100" value="50" aria-label="Volume slider">
              <span class="volume-icon volume-icon-max">${iconVolume}</span>
            </div>
          </div>
        </div>
        <div id="clock-display" class="clock-display" aria-live="polite"></div>
      </div>
    </footer>
  </main>
`;

const requireElement = <T extends Element>(selector: string): T => {
  const element = document.querySelector<T>(selector);
  if (!element) {
    throw new Error('Required UI elements could not be initialized.');
  }
  return element;
};

export const desktop = requireElement<HTMLDivElement>('#desktop');
export const hamburgerMenu = requireElement<HTMLButtonElement>('#hamburger-menu');
export const menuPopup = requireElement<HTMLDivElement>('#menu-popup');
export const menuNewTerminal = requireElement<HTMLButtonElement>('#menu-new-terminal');
export const menuFile = requireElement<HTMLButtonElement>('#menu-file');
export const menuAbout = requireElement<HTMLButtonElement>('#menu-about');
export const taskbarCenter = requireElement<HTMLDivElement>('#taskbar-center');
export const clockDisplay = requireElement<HTMLDivElement>('#clock-display');
export const taskbarVolumeButton = requireElement<HTMLButtonElement>('#taskbar-volume-button');
export const volumePopup = requireElement<HTMLDivElement>('#volume-popup');
export const volumeSlider = requireElement<HTMLInputElement>('#volume-slider');
export const volumeDisplay = requireElement<HTMLDivElement>('#volume-display');

// Initialize start menu items with icons
menuNewTerminal.innerHTML = `<span class="menu-item-icon">${iconTerminal}</span><span class="menu-item-label">ターミナル</span>`;
menuFile.innerHTML = `<span class="menu-item-icon">${iconFolder}</span><span class="menu-item-label">ファイル</span>`;
menuAbout.innerHTML = `<span class="menu-item-icon">${iconCategory}</span><span class="menu-item-label">About...</span>`;

document.documentElement.style.setProperty('--taskbar-height', `${TASKBAR_HEIGHT_PX}px`);
document.documentElement.style.setProperty('--titlebar-height', `${TITLE_BAR_HEIGHT}px`);

// Small controls on the taskbar: clock, start menu and volume
import iconMenu from '../assets/icons/box-multiple.svg?raw';
import iconVolume from '../assets/icons/volume.svg?raw';
import iconVolume2 from '../assets/icons/volume-2.svg?raw';
import iconVolume3 from '../assets/icons/volume-3.svg?raw';
import iconVolume4 from '../assets/icons/volume-4.svg?raw';
import { getGlobalVolume, resumeAudioContext, setGlobalVolume, setVolumeChangeListener } from '../audio/audio';
import {
  clockDisplay,
  desktop,
  hamburgerMenu,
  menuAbout,
  menuFile,
  menuNewTerminal,
  menuPopup,
  taskbarVolumeButton,
  volumeDisplay,
  volumePopup,
  volumeSlider,
} from './dom';

// ---- Clock

const formatClock = (date: Date): string => {
  return new Intl.DateTimeFormat('ja-JP', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(date);
};

const updateClock = (): void => {
  clockDisplay.textContent = formatClock(new Date());
};

export const initClock = (): void => {
  updateClock();
  window.setInterval(updateClock, 1000);
};

// ---- Start menu

let menuOpen = false;

export const isMenuOpen = (): boolean => {
  return menuOpen;
};

export const closeMenu = (): void => {
  menuOpen = false;
  menuPopup.classList.add('hidden');
};

const toggleMenu = (): void => {
  menuOpen = !menuOpen;
  if (menuOpen) {
    menuPopup.classList.remove('hidden');
  } else {
    menuPopup.classList.add('hidden');
  }
};

export type StartMenuActions = {
  openTerminal: () => void;
  openFileManager: () => void;
  openAbout: () => void;
};

export const initStartMenu = (actions: StartMenuActions): void => {
  hamburgerMenu.innerHTML = iconMenu;
  hamburgerMenu.addEventListener('click', toggleMenu);

  menuNewTerminal.addEventListener('click', () => {
    actions.openTerminal();
    closeMenu();
  });

  menuFile.addEventListener('click', () => {
    actions.openFileManager();
    closeMenu();
  });

  menuAbout.addEventListener('click', () => {
    actions.openAbout();
    closeMenu();
  });

  desktop.addEventListener('click', (event) => {
    if (menuOpen && event.target !== hamburgerMenu && !menuPopup.contains(event.target as Node)) {
      closeMenu();
    }
  });
};

// ---- Volume

let volumePopupVisible = false;

const updateVolumeIcon = (): void => {
  if (!taskbarVolumeButton) {
    return;
  }

  let iconHtml: string;

  // Show icon based on volume level (regardless of AudioContext state)
  const volumePercent = getGlobalVolume() * 100;
  if (volumePercent === 0) {
    // Muted (0%)
    iconHtml = iconVolume3;
  } else if (volumePercent >= 70) {
    // Large (9~10)
    iconHtml = iconVolume;
  } else if (volumePercent >= 30) {
    // Medium (4~8)
    iconHtml = iconVolume2;
  } else {
    // Small (1~3)
    iconHtml = iconVolume4;
  }

  taskbarVolumeButton.innerHTML = iconHtml;
};

const handleVolumeSliderChange = (e: Event): void => {
  const slider = e.target as HTMLInputElement;
  const volumeValue = parseInt(slider.value, 10);
  const volume = volumeValue / 100;
  setGlobalVolume(volume);
  // Update the display value
  volumeDisplay.textContent = String(volumeValue);
};

const handleVolumeButtonClick = (e: MouseEvent): void => {
  e.stopPropagation();
  volumePopupVisible = !volumePopupVisible;

  if (volumePopupVisible) {
    volumePopup.classList.remove('hidden');
    const volumePercent = Math.round(getGlobalVolume() * 100);
    volumeSlider.value = String(volumePercent);
    volumeDisplay.textContent = String(volumePercent);
    // Try to resume AudioContext
    resumeAudioContext();
  } else {
    volumePopup.classList.add('hidden');
  }
};

const closeVolumePopup = (): void => {
  volumePopupVisible = false;
  volumePopup.classList.add('hidden');
};

export const initVolumeControl = (): void => {
  setVolumeChangeListener(updateVolumeIcon);

  // Set up event listeners for volume control
  taskbarVolumeButton.addEventListener('click', handleVolumeButtonClick);
  volumeSlider.addEventListener('change', handleVolumeSliderChange);
  volumeSlider.addEventListener('input', handleVolumeSliderChange);

  // Initialize volume icon on startup
  updateVolumeIcon();

  // Close volume popup when clicking outside
  document.addEventListener('click', (e: MouseEvent) => {
    if (volumePopupVisible && !taskbarVolumeButton.contains(e.target as Node) && !volumePopup.contains(e.target as Node)) {
      closeVolumePopup();
    }
  });
};

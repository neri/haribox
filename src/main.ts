import './style.css';
import './shell/dom';
import { createAboutWindow, registerAboutKind } from './apps/about';
import { registerCanvasKind } from './apps/canvas';
import { initDropImport, registerSystemModalKind } from './apps/dropImport';
import { createFileManagerWindow, registerFileManagerKind } from './apps/fileManager';
import { createOnboardingWindow, registerOnboardingKind } from './apps/onboarding';
import { createTerminalWindow, registerTerminalKind } from './apps/terminal/panel';
import { registerTextViewerKind } from './apps/textViewer';
import { loadSavedVolume } from './audio/audio';
import { loadFileSystem, setStorageWarningHandler } from './fs/fileSystem';
import { initKeyboard } from './input/keyboard';
import { initTaskbar } from './shell/taskbar';
import { initClock, initStartMenu, initVolumeControl } from './shell/taskbarWidgets';
import { initTaskRunner } from './task/taskRunner';

// Shell
initClock();
loadSavedVolume();
initVolumeControl();
setStorageWarningHandler((message) => window.alert(message));

// Window kinds
registerTerminalKind();
registerCanvasKind();
registerFileManagerKind();
registerAboutKind();
registerOnboardingKind();
registerSystemModalKind();
registerTextViewerKind();

// Input and background services
initTaskRunner();
initDropImport();
initStartMenu({
  openTerminal: createTerminalWindow,
  openFileManager: createFileManagerWindow,
  openAbout: createAboutWindow,
});
initTaskbar();
initKeyboard();

// Initial desktop
loadFileSystem();
createTerminalWindow();
createFileManagerWindow();
createOnboardingWindow();

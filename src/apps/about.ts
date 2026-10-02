import iconCategory from '../assets/icons/category.svg?raw';
import { APP_IDS, APP_VERSION, GIT_HASH } from '../constants';
import { registerWindowKind } from '../wm/registry';
import { renderWindows } from '../wm/render';
import { state } from '../wm/state';
import type { WindowId, WindowModel } from '../wm/state';
import {
  bringToFrontIfNeeded,
  closeWindow,
  createWindowId,
  createWindowModel,
  findWindowById,
  getCenteredWindowPosition,
} from '../wm/windowManager';

// Markdown text to HTML converter (supports **bold** formatting)
function renderMarkdown(text: string): string {
  return text
    .split('\n')
    .map(line => {
      // Replace **text** with <strong>text</strong>
      return line.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    })
    .join('<br>');
}

// MIT License text
const MIT_LICENSE_TEXT = `**MIT License**

**Copyright (c) 2026 Nerry**

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

----

**Apps**:

**KL-01**

**川合堂ライセンス-01  ver.1.0  2000.12.30 H.Kawai (川合秀実)**

  川合秀実URL     http://k.osask.jp/
       e-mail     kawai@osask.jp

----

**CrystalCPUID for HariboteOS**

**KL-01**

Copyright (c) 2007 hiyohiyo (Project HiyOS) https://crystalmark.info/

----

**Icons**:

**Tabler Icons (https://tabler-icons.io/)**

**MIT License**

**Copyright (c) 2020-2026 Paweł Kuna**

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

----

**Web UI**:
Built with GitHub Copilot

----

**Special thanks to**:

* megumish🍕
* wataash
* NiwakaDev
`;

let aboutWindowId: WindowId | null = null;

const createAboutPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'about-panel';

  // App info container
  const infoContainer = document.createElement('div');
  infoContainer.className = 'about-info';

  const appNameDiv = document.createElement('div');
  appNameDiv.className = 'about-app-name';
  appNameDiv.innerHTML = '<strong>HariboteBox</strong>';
  infoContainer.appendChild(appNameDiv);

  const versionDiv = document.createElement('div');
  versionDiv.className = 'about-version';
  versionDiv.textContent = `v${APP_VERSION} | ${GIT_HASH}`;
  infoContainer.appendChild(versionDiv);

  // License text container
  const licenseContainer = document.createElement('div');
  licenseContainer.className = 'about-license-container';

  const licenseLabel = document.createElement('label');
  licenseLabel.htmlFor = 'about-license-text';
  licenseLabel.textContent = 'License:';

  const licenseDiv = document.createElement('div');
  licenseDiv.id = 'about-license-text';
  licenseDiv.className = 'about-license-text';
  licenseDiv.innerHTML = renderMarkdown(MIT_LICENSE_TEXT);
  licenseDiv.setAttribute('role', 'textbox');
  licenseDiv.setAttribute('aria-label', 'MIT License');
  licenseDiv.style.whiteSpace = 'pre-wrap';
  licenseDiv.style.wordWrap = 'break-word';
  licenseDiv.style.overflowY = 'auto';
  licenseDiv.style.overflowX = 'hidden';
  licenseDiv.style.padding = '8px';

  licenseContainer.append(licenseLabel, licenseDiv);

  // OK button container
  const buttonContainer = document.createElement('div');
  buttonContainer.className = 'about-button-container';

  const githubButton = document.createElement('button');
  githubButton.type = 'button';
  githubButton.textContent = 'GitHub';
  githubButton.className = 'about-github-button window-secondary-button';
  githubButton.addEventListener('click', (event) => {
    event.stopPropagation();
    window.open('https://github.com/neri/haribox/', '_blank');
  });

  const okButton = document.createElement('button');
  okButton.type = 'button';
  okButton.textContent = 'OK';
  okButton.className = 'about-ok-button window-primary-button';
  okButton.addEventListener('click', (event) => {
    event.stopPropagation();
    closeWindow(win.id);
  });

  buttonContainer.append(githubButton, okButton);

  panel.append(infoContainer, licenseContainer, buttonContainer);
  return panel;
};

const ABOUT_WIDTH = 480;
const ABOUT_HEIGHT = 380;

export const createAboutWindow = (): void => {
  // If about window already exists, bring it to front
  if (aboutWindowId && findWindowById(aboutWindowId)) {
    bringToFrontIfNeeded(aboutWindowId);
    return;
  }

  const id = createWindowId();
  const centered = getCenteredWindowPosition(ABOUT_WIDTH, ABOUT_HEIGHT);

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.ABOUT,
    kind: 'about',
    title: 'About',
    x: centered.x,
    y: centered.y,
    width: ABOUT_WIDTH,
    height: ABOUT_HEIGHT,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  aboutWindowId = id;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
};

export const registerAboutKind = (): void => {
  registerWindowKind('about', {
    modal: true,
    icon: iconCategory,
    typeLabel: 'About',
    renderContent: createAboutPanel,
    onClosing: (win) => {
      if (aboutWindowId === win.id) {
        aboutWindowId = null;
      }
    },
  });
};

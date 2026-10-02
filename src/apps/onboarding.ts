import iconAppWindow from '../assets/icons/app-window.svg?raw';
import blissImage from '../assets/bliss.png';
import { APP_IDS } from '../constants';
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

let onboardingWindowId: WindowId | null = null;

const createOnboardingPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'onboarding-panel';
  panel.style.backgroundImage = `url(${blissImage})`;
  panel.style.backgroundSize = 'cover';
  panel.style.backgroundPosition = 'center';
  panel.style.backgroundRepeat = 'no-repeat';

  // Container for content
  const contentContainer = document.createElement('div');
  contentContainer.className = 'onboarding-content';

  // Title
  const title = document.createElement('h2');
  title.className = 'onboarding-title';
  title.textContent = 'ようこそ！';
  contentContainer.appendChild(title);

  // Description
  const description = document.createElement('p');
  description.className = 'onboarding-description';
  description.innerHTML = `<a href="http://hrb.osask.jp/" target="_blank" rel="noopener noreferrer">はりぼてOS</a> のアプリをブラウザー上で実行できるデスクトップ環境エミュレーターです。<br>
現在ベータ運用中です。<br>
内蔵アプリは概ね検証済みですが、外部のアプリはうまく動作しない場合があります。`;
  contentContainer.appendChild(description);

  // Features list
  const features = document.createElement('ul');
  features.className = 'onboarding-features';

  const featuresList = [
    '一般的なウィンドウシステムと同様にウィンドウ操作できます。',
    'ファイル一覧から HRB ファイルを選択してGUIアプリを実行できます。',
    '一部のアプリはターミナルでコマンド入力が必要なものがあります。',
    'ターミナルでは HELP コマンドで使用可能なコマンドを確認できます。',
    'ドラッグアンドドロップで外部の HRB ファイルを localStorage に取り込めます。',
  ];

  featuresList.forEach(feature => {
    const li = document.createElement('li');
    li.className = 'onboarding-feature-item';
    li.textContent = feature;
    features.appendChild(li);
  });

  contentContainer.appendChild(features);

  // Button container
  const buttonContainer = document.createElement('div');
  buttonContainer.className = 'onboarding-button-container';

  const closeButton = document.createElement('button');
  closeButton.type = 'button';
  closeButton.textContent = '開始する';
  closeButton.className = 'onboarding-close-button window-primary-button';
  closeButton.addEventListener('click', (event) => {
    event.stopPropagation();
    closeWindow(win.id);
  });

  buttonContainer.appendChild(closeButton);
  contentContainer.appendChild(buttonContainer);

  panel.appendChild(contentContainer);
  return panel;
};

export const createOnboardingWindow = (): void => {
  // If onboarding window already exists, bring it to front
  if (onboardingWindowId && findWindowById(onboardingWindowId)) {
    bringToFrontIfNeeded(onboardingWindowId);
    return;
  }

  const id = createWindowId();
  const centered = getCenteredWindowPosition(640, 546);

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.ONBOARDING,
    kind: 'onboarding',
    title: 'ようこそ',
    x: centered.x,
    y: centered.y,
    width: 640,
    height: 546,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  onboardingWindowId = id;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
};

export const registerOnboardingKind = (): void => {
  registerWindowKind('onboarding', {
    icon: iconAppWindow,
    typeLabel: 'Task',
    renderContent: createOnboardingPanel,
    onClosing: (win) => {
      if (onboardingWindowId === win.id) {
        onboardingWindowId = null;
      }
    },
  });
};

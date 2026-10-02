import { getWindowKindDefinition } from '../wm/registry';
import { state } from '../wm/state';
import { bringToFrontIfNeeded, findWindowById } from '../wm/windowManager';
import { desktop, taskbarCenter } from './dom';

/**
 * タスクバーボタンをレンダリング
 */
export const renderTaskbarButtons = (): void => {
  taskbarCenter.innerHTML = '';

  // 既存のドロップダウンメニューをクリーンアップ
  document.querySelectorAll('.taskbar-dropdown-menu').forEach(el => el.remove());

  state.windowGroups.forEach((group) => {
    const groupContainer = document.createElement('div');
    groupContainer.className = 'taskbar-button-group';

    // グループボタン（Canvas は個別表示なので、非Canvas のみグループ化）
    const isCanvasGroup = group.kind === 'canvas';
    const isGrouped = group.windowIds.length > 1 && !isCanvasGroup;

    const groupButton = document.createElement('button');
    groupButton.type = 'button';
    groupButton.className = 'taskbar-button';

    // アクティブウィンドウがこのグループに属するかチェック
    const isGroupActive = state.activeWindowId && group.windowIds.includes(state.activeWindowId);
    if (isGroupActive) {
      groupButton.classList.add('taskbar-button-active');
    }

    const iconSpan = document.createElement('span');
    iconSpan.className = 'taskbar-button-icon';
    iconSpan.innerHTML = getWindowKindDefinition(group.kind).icon;
    groupButton.appendChild(iconSpan);

    if (isGrouped) {
      // グループボタン表示
      const labelSpan = document.createElement('span');
      labelSpan.className = 'taskbar-button-label';
      labelSpan.textContent = `${getWindowKindDefinition(group.kind).typeLabel} ×${group.windowIds.length}`;
      groupButton.appendChild(labelSpan);

      groupButton.addEventListener('click', () => {
        group.isExpanded = !group.isExpanded;
        renderTaskbarButtons();
      });

      groupContainer.appendChild(groupButton);

      // ドロップダウンメニュー
      if (group.isExpanded) {
        const dropdown = document.createElement('div');
        dropdown.className = 'taskbar-dropdown-menu';

        group.windowIds.forEach((windowId) => {
          const win = findWindowById(windowId);
          if (!win) return;

          const item = document.createElement('button');
          item.type = 'button';
          item.className = 'taskbar-dropdown-item';
          if (win.isActive) {
            item.classList.add('taskbar-dropdown-item-active');
          }
          item.textContent = win.title;

          item.addEventListener('click', () => {
            bringToFrontIfNeeded(windowId);
            group.isExpanded = false;
            renderTaskbarButtons();
          });

          dropdown.appendChild(item);
        });

        // ドロップダウンメニューを body に追加（fixed positioning用）
        document.body.appendChild(dropdown);

        // DOM に追加された後、位置を計算
        // (offsetHeight がまだ計算されていないことがあるため、setTimeout で遅延実行)
        setTimeout(() => {
          const groupButtonRect = groupButton.getBoundingClientRect();
          dropdown.style.position = 'fixed';
          dropdown.style.left = `${groupButtonRect.left}px`;
          dropdown.style.top = `${groupButtonRect.top - dropdown.offsetHeight - 4}px`;
        }, 0);
      }
    } else {
      // 個別ボタン表示
      const win = findWindowById(group.windowIds[0]);
      if (win) {
        // アクティブウィンドウかチェック
        if (win.isActive) {
          groupButton.classList.add('taskbar-button-active');
        }

        const labelSpan = document.createElement('span');
        labelSpan.className = 'taskbar-button-label';
        labelSpan.textContent = win.title;
        groupButton.appendChild(labelSpan);

        groupButton.addEventListener('click', () => {
          bringToFrontIfNeeded(win.id);
        });
      }

      groupContainer.appendChild(groupButton);
    }

    taskbarCenter.appendChild(groupContainer);
  });
};

export const initTaskbar = (): void => {
  desktop.addEventListener('click', (event) => {
    // ドロップダウンを閉じる
    if (!(event.target as Element).closest('.taskbar-button-group') && state.windowGroups.some(g => g.isExpanded)) {
      state.windowGroups.forEach(g => g.isExpanded = false);
      renderTaskbarButtons();
    }
  });
};

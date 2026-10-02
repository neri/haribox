import type { WindowGroupInfo, WindowKind, WindowModel } from './state';

/**
 * ウィンドウグループ化関数：ウィンドウタイプごとにグループ化情報を計算
 * Canvas タイプは個別表示、その他のタイプはグループ化
 */
export const groupWindowsByKind = (windows: WindowModel[]): WindowGroupInfo[] => {
  const grouped = new Map<string, WindowModel[]>();
  const canvasWindows: WindowModel[] = [];

  for (const win of windows) {
    if (win.kind === 'canvas') {
      canvasWindows.push(win);
    } else {
      const key = win.kind;
      if (!grouped.has(key)) {
        grouped.set(key, []);
      }
      grouped.get(key)!.push(win);
    }
  }

  const groups: WindowGroupInfo[] = [];

  // 非Canvas ウィンドウをグループ化
  for (const [kind, windowList] of grouped) {
    const appId = windowList[0].appId;
    groups.push({
      appId,
      kind: kind as WindowKind,
      windowIds: windowList.map(w => w.id),
      isExpanded: false,
    });
  }

  // Canvas ウィンドウは個別表示
  for (const canvasWin of canvasWindows) {
    groups.push({
      appId: canvasWin.appId,
      kind: 'canvas',
      windowIds: [canvasWin.id],
      isExpanded: false,
    });
  }

  return groups;
};

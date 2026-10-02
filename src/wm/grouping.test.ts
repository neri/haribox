import { describe, expect, it } from 'vitest';
import { groupWindowsByKind } from './grouping';
import type { WindowKind, WindowModel } from './state';

const win = (id: string, kind: WindowKind, appId: string): WindowModel => ({
  id,
  appId,
  kind,
  title: id,
  x: 0,
  y: 0,
  width: 100,
  height: 100,
  zIndex: 1,
  isActive: false,
});

describe('groupWindowsByKind', () => {
  it('returns no groups for no windows', () => {
    expect(groupWindowsByKind([])).toEqual([]);
  });

  it('groups non-canvas windows by kind and lists canvas windows individually after them', () => {
    const groups = groupWindowsByKind([
      win('t1', 'terminal', 'app-terminal'),
      win('c1', 'canvas', 'app-canvas'),
      win('f1', 'filemanager', 'app-files'),
      win('t2', 'terminal', 'app-terminal'),
      win('c2', 'canvas', 'app-canvas'),
    ]);

    expect(groups).toEqual([
      { appId: 'app-terminal', kind: 'terminal', windowIds: ['t1', 't2'], isExpanded: false },
      { appId: 'app-files', kind: 'filemanager', windowIds: ['f1'], isExpanded: false },
      { appId: 'app-canvas', kind: 'canvas', windowIds: ['c1'], isExpanded: false },
      { appId: 'app-canvas', kind: 'canvas', windowIds: ['c2'], isExpanded: false },
    ]);
  });
});

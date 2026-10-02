export type WindowId = string;
export type AppId = string;
export type WindowKind = 'canvas' | 'terminal' | 'filemanager' | 'about' | 'textviewer' | 'onboarding' | 'systemmodal';

export type WindowModel = {
  id: WindowId;
  appId: AppId;
  kind: WindowKind;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
  isActive: boolean;
};

export type WindowGroupInfo = {
  appId: AppId;
  kind: WindowKind;
  windowIds: WindowId[];
  isExpanded: boolean;
};

export const state: {
  windows: WindowModel[];
  nextZIndex: number;
  activeWindowId: WindowId | null;
  windowGroups: WindowGroupInfo[];
} = {
  windows: [],
  nextZIndex: 1,
  activeWindowId: null,
  windowGroups: [],
};

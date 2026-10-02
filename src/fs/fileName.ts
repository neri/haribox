/**
 * ファイルシステムのバリデーション・正規化ルール
 * 
 * 許可される文字: A-Z, a-z, 0-9, '.', '_', '!'
 * 最大ファイル名長: 31文字（初期FSとの互換性）
 * 
 * バリデーション処理:
 * 1. パス区切り文字 (/ \) を除去してファイル名のみ抽出
 * 2. 非ASCII文字を下線に置換
 * 3. 非ASCII文字が50%以上の場合は処理失敗
 * 4. 連続する下線をまとめ、先頭・末尾のドットを削除（パストラバーサル対策）
 * 5. ファイル名長が31文字を超える場合はトリミング（拡張子を優先保持）
 */
export const normalizePathLikeName = (input: string): string => {
  return input.split(/[\\/]/).filter(Boolean).at(-1) ?? '';
};

export const toCanonicalFileKey = (name: string): string => {
  return name.toUpperCase();
};

export type NormalizeFileNameResult = { ok: true; name: string } | { ok: false; reason: string };

// 最大ファイル名長（初期FSとの互換性）
export const MAX_FILE_NAME_LENGTH = 31;
// 名前が長い場合のトリミング時に保持する最小ベース名長
const MIN_BASE_NAME_LENGTH = 8;

const splitBaseAndExtension = (name: string): { base: string; ext: string } => {
  const dotIndex = name.lastIndexOf('.');
  // ドットが先頭にある、または末尾にある場合は拡張子として扱わない
  if (dotIndex <= 0 || dotIndex === name.length - 1) {
    return { base: name, ext: '' };
  }

  return {
    base: name.slice(0, dotIndex),
    ext: name.slice(dotIndex + 1),
  };
};

// ファイル名のサニタイズ
// 許可文字：英数字、アンダースコア、ドット、感嘆符
// 非ASCII文字は下線に置換
const sanitizeAsciiFileName = (name: string): { sanitized: string; nonAsciiCount: number } => {
  let nonAsciiCount = 0;
  let sanitized = '';

  for (const char of name) {
    const code = char.charCodeAt(0);
    if (code > 127) {
      nonAsciiCount += 1;
      sanitized += '_';
      continue;
    }

    // 許可文字: A-Z, a-z, 0-9, '.', '_', '!'
    if (/[A-Za-z0-9_.!]/.test(char)) {
      sanitized += char;
      continue;
    }

    sanitized += '_';
  }

  return { sanitized, nonAsciiCount };
};

// ファイル名の正規化と検証
// 1. パス区切り文字を除去
// 2. 許可文字以外を下線に置換
// 3. 非ASCII文字が50%以上でないかチェック
// 4. 連続する下線をまとめ、先頭・末尾のドットを削除
// 5. 長さが31文字を超える場合はトリミング
export const normalizeFileName = (input: string): NormalizeFileNameResult => {
  const raw = normalizePathLikeName(input.trim());
  if (!raw) {
    return { ok: false, reason: 'Filename is empty.' };
  }

  const { sanitized, nonAsciiCount } = sanitizeAsciiFileName(raw);
  // 非ASCII文字が50%以上含まれている場合は拒否
  if (nonAsciiCount > 0 && nonAsciiCount * 2 > raw.length) {
    return {
      ok: false,
      reason: `Filename conversion failed: too many non-ASCII characters in ${raw}`,
    };
  }

  // 連続下線をまとめ、先頭・末尾のドットを除去（パストラバーサル対策）
  const collapsed = sanitized.replace(/_+/g, '_').replace(/^\.+/, '').replace(/\.+$/, '');
  if (!collapsed) {
    return { ok: false, reason: `Filename conversion failed: ${raw}` };
  }

  const { base: rawBase, ext: rawExt } = splitBaseAndExtension(collapsed);
  const base = rawBase || '_';
  const ext = rawExt;

  if (collapsed.length <= MAX_FILE_NAME_LENGTH) {
    return { ok: true, name: collapsed };
  }

  // ファイル名が長すぎる場合のトリミング処理
  if (!ext) {
    // 拡張子がない場合はベース名だけをトリミング
    return { ok: true, name: base.slice(0, MAX_FILE_NAME_LENGTH) };
  }

  // 拡張子が長くない場合は拡張子を保持してベース名をトリミング
  const baseLimitWhenKeepingFullExt = MAX_FILE_NAME_LENGTH - 1 - ext.length;
  if (baseLimitWhenKeepingFullExt >= MIN_BASE_NAME_LENGTH) {
    return {
      ok: true,
      name: `${base.slice(0, baseLimitWhenKeepingFullExt)}.${ext}`,
    };
  }

  // ベース名の最小長も満たせない場合は拡張子もトリミング
  const extLimit = Math.max(0, MAX_FILE_NAME_LENGTH - 1 - MIN_BASE_NAME_LENGTH);
  return {
    ok: true,
    name: `${base.slice(0, MIN_BASE_NAME_LENGTH)}.${ext.slice(0, extLimit)}`,
  };
};

import { readFile, readdir, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import zlib from 'node:zlib';
import { promisify } from 'node:util';

const deflate = promisify(zlib.deflate);

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');
const sourceDir = path.join(projectRoot, 'src', 'initial-fs');
const outFile = path.join(projectRoot, 'src', 'generated-initial-fs.ts');

const collectFiles = async (dir) => {
    const entries = await readdir(dir, { withFileTypes: true });
    const files = [];

    for (const entry of entries) {
        const entryPath = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            files.push(...(await collectFiles(entryPath)));
            continue;
        }

        if (entry.isFile()) {
            files.push(entryPath);
        }
    }

    return files;
};

const toBase64 = (bytes) => {
    return Buffer.from(bytes).toString('base64');
};

const main = async () => {
    const files = await collectFiles(sourceDir);
    const entries = [];

    for (const filePath of files.sort()) {
        const relativePath = path.relative(sourceDir, filePath).split(path.sep).join('/');
        const content = await readFile(filePath);
        const compressed = await deflate(content);
        entries.push({ name: relativePath, contentBase64: toBase64(compressed) });
    }

    const content = `export type InitialFsEntry = { name: string; contentBase64: string };

export const INITIAL_FS_ENTRIES: InitialFsEntry[] = ${JSON.stringify(entries, null, 2)};
`;

    await mkdir(path.dirname(outFile), { recursive: true });
    await writeFile(outFile, content, 'utf8');
};

await main();

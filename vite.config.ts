import { defineConfig } from 'vite';
import { execSync } from 'node:child_process';

const resolveGitHash = (): string => {
    if (process.env.GITHUB_SHA) {
        return process.env.GITHUB_SHA.slice(0, 7);
    }

    try {
        return execSync('git rev-parse --short HEAD', { stdio: ['ignore', 'pipe', 'ignore'] })
            .toString()
            .trim();
    } catch {
        return 'unknown';
    }
};

export default defineConfig(() => {
    const repoName = process.env.GITHUB_REPOSITORY?.split('/')[1];
    const isGitHubActionsBuild = process.env.GITHUB_ACTIONS === 'true' && Boolean(repoName);
    const appVersion = process.env.npm_package_version ?? '0.0.0';
    const gitHash = resolveGitHash();

    return {
        base: isGitHubActionsBuild ? `/${repoName}/` : '/',
        define: {
            __APP_VERSION__: JSON.stringify(appVersion),
            __GIT_HASH__: JSON.stringify(gitHash),
        },
        resolve: {
            alias: {
                'env': new URL('./src/wasm/env.ts', import.meta.url).pathname,
            },
        },
    };
});
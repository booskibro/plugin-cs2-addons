// Mirrors the shared GameAP plugin build (see plugin-tickets/frontend):
// panel-provided libraries are externalized and rewritten to window.* globals.

import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { resolve } from 'path';

function globalExternalsPlugin() {
    const globals = {
        'vue': 'window.Vue',
        'vue-router': 'window.VueRouter',
        'pinia': 'window.Pinia',
        'axios': 'window.axios',
        'naive-ui': 'window.NaiveUI',
    };

    return {
        name: 'global-externals',
        renderChunk(code) {
            let result = code;
            for (const [moduleId, globalVar] of Object.entries(globals)) {
                const importRegex = new RegExp(
                    `import\\s*\\{([^}]+)\\}\\s*from\\s*["']${moduleId}["'];?`,
                    'g'
                );
                result = result.replace(importRegex, (_, imports) => {
                    const importList = imports.split(',').map(i => i.trim());
                    const assignments = importList.map(i => {
                        const parts = i.split(/\s+as\s+/);
                        const original = parts[0].trim();
                        const alias = parts.length === 2 ? parts[1].trim() : original;
                        return `${alias} = ${globalVar}?.${original}`;
                    }).join(', ');
                    return `const ${assignments};`;
                });

                const importStarRegex = new RegExp(
                    `import\\s*\\*\\s*as\\s*(\\w+)\\s*from\\s*["']${moduleId}["'];?`,
                    'g'
                );
                result = result.replace(importStarRegex, (_, name) => {
                    return `const ${name} = ${globalVar};`;
                });

                const importDefaultRegex = new RegExp(
                    `import\\s+(\\w+)\\s*from\\s*["']${moduleId}["'];?`,
                    'g'
                );
                result = result.replace(importDefaultRegex, (_, name) => {
                    return `const ${name} = ${globalVar};`;
                });
            }
            return { code: result, map: null };
        }
    };
}

function wrapInIIFEPlugin() {
    return {
        name: 'wrap-iife',
        generateBundle(options, bundle) {
            for (const fileName of Object.keys(bundle)) {
                const chunk = bundle[fileName];
                if (chunk.type === 'chunk' && chunk.code) {
                    const exportMatch = chunk.code.match(/export\s*\{\s*(\w+)\s+as\s+(\w+)\s*\};?\s*$/s);
                    if (exportMatch) {
                        const [fullExport, internalName, exportedName] = exportMatch;
                        const codeWithoutExport = chunk.code.replace(fullExport, '').trim();
                        chunk.code = `const ${exportedName} = (function() {\n${codeWithoutExport}\nreturn ${internalName};\n})();\nexport { ${exportedName} };`;
                    }
                }
            }
        }
    };
}

export default defineConfig({
    plugins: [vue()],
    build: {
        lib: {
            entry: resolve(process.cwd(), 'src/index.ts'),
            formats: ['es'],
            fileName: () => 'plugin.js',
            cssFileName: 'plugin',
        },
        outDir: 'dist',
        emptyOutDir: true,
        rollupOptions: {
            external: ['vue', 'vue-router', 'pinia', 'axios', 'naive-ui'],
            output: {
                globals: {
                    vue: 'Vue',
                    'vue-router': 'VueRouter',
                    pinia: 'Pinia',
                    axios: 'axios',
                    'naive-ui': 'NaiveUI',
                },
            },
            plugins: [globalExternalsPlugin(), wrapInIIFEPlugin()],
        },
    },
    resolve: {
        alias: {
            '@': resolve(process.cwd(), 'src'),
        },
    },
});

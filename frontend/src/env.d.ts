/// <reference types="vite/client" />

declare module '*.vue' {
    import type { DefineComponent } from 'vue';
    const component: DefineComponent<object, object, unknown>;
    export default component;
}

interface Window {
    $dialog: {
        success: (options: Record<string, unknown>) => void;
        error: (options: Record<string, unknown>) => void;
        info: (options: Record<string, unknown>) => void;
    };
    $message: {
        success: (text: string) => void;
        error: (text: string) => void;
        info: (text: string) => void;
        warning: (text: string) => void;
    };
}

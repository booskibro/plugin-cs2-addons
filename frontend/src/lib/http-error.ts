// Duck-typed HTTP error inspection.
//
// The panel externalizes `axios` to window.axios, which is an axios INSTANCE
// (axios.create()); instances carry none of the namespace statics, so
// `axios.isAxiosError` is `undefined` at runtime and calling it crashes the
// error handler that was trying to explain another failure. Axios rejection
// objects always carry `isAxiosError: true` and/or `response`/`request` on
// the error itself, so shape-checking needs nothing from the namespace.

export interface HttpErrorLike {
    message?: string;
    response?: {
        status?: number;
        data?: unknown;
    };
}

/** The error as an HTTP-transport error, or null for non-HTTP failures. */
export function asHttpError(error: unknown): HttpErrorLike | null {
    if (typeof error !== 'object' || error === null) {
        return null;
    }
    const candidate = error as { isAxiosError?: unknown; response?: unknown; request?: unknown };
    if (candidate.isAxiosError === true || 'response' in candidate || 'request' in candidate) {
        return error as HttpErrorLike;
    }
    return null;
}

/** HTTP status of the failed response, when there was a response at all. */
export function httpStatus(error: unknown): number | undefined {
    return asHttpError(error)?.response?.status;
}

/** The backend body's "message" field, when the response carried one. */
export function httpBodyMessage(error: unknown): string | undefined {
    const data = asHttpError(error)?.response?.data;
    if (typeof data === 'object' && data !== null) {
        const message = (data as { message?: unknown }).message;
        if (typeof message === 'string' && message.trim() !== '') {
            return message;
        }
    }
    return undefined;
}

import { describe, expect, it } from 'vitest';

import { asHttpError, httpBodyMessage, httpStatus } from '../lib/http-error';

// Shaped like a real axios rejection: the flag and response live on the error
// object itself, no namespace statics involved.
const axiosLike = {
    isAxiosError: true,
    message: 'Request failed with status code 422',
    response: {
        status: 422,
        data: { message: 'rcon authentication failed' },
    },
};

describe('asHttpError', () => {
    it('recognizes axios-shaped rejections', () => {
        expect(asHttpError(axiosLike)).not.toBeNull();
    });

    it('recognizes network errors (request, no response)', () => {
        expect(asHttpError({ isAxiosError: true, message: 'Network Error', request: {} })).not.toBeNull();
        expect(asHttpError({ request: {}, message: 'timeout' })).not.toBeNull();
    });

    it('rejects plain runtime errors and primitives', () => {
        expect(asHttpError(new TypeError('x is not a function'))).toBeNull();
        expect(asHttpError('boom')).toBeNull();
        expect(asHttpError(null)).toBeNull();
        expect(asHttpError(undefined)).toBeNull();
    });
});

describe('httpStatus', () => {
    it('reads the response status', () => {
        expect(httpStatus(axiosLike)).toBe(422);
    });

    it('is undefined without a response', () => {
        expect(httpStatus({ isAxiosError: true, request: {} })).toBeUndefined();
        expect(httpStatus(new Error('nope'))).toBeUndefined();
    });
});

describe('httpBodyMessage', () => {
    it('reads the backend message field', () => {
        expect(httpBodyMessage(axiosLike)).toBe('rcon authentication failed');
    });

    it('ignores empty and non-string messages', () => {
        expect(
            httpBodyMessage({ isAxiosError: true, response: { status: 500, data: { message: '  ' } } }),
        ).toBeUndefined();
        expect(
            httpBodyMessage({ isAxiosError: true, response: { status: 500, data: 'plain text' } }),
        ).toBeUndefined();
        expect(httpBodyMessage({ isAxiosError: true, response: { status: 500 } })).toBeUndefined();
    });
});

declare module 'encoding-japanese' {
    interface ConvertOptions {
        to: string;
        from: string;
    }

    const Encoding: {
        detect(bytes: Uint8Array): string | boolean;
        convert(bytes: Uint8Array, options: ConvertOptions): string;
    };

    export default Encoding;
}

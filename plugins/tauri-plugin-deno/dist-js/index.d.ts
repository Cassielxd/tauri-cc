import { Channel } from "@tauri-apps/api/core";
export interface IpcMessage {
    name: string;
    rid: number;
    content: any;
}
export declare function sendToDeno(value: IpcMessage): Promise<string | null>;
export declare function listenOn(rid: number, name: string): Promise<string>;
export declare function unlistenFrom(rid: number, name: string): Promise<string | null>;
export declare function createDenoChannel(key: string, channel: Channel<any>): Promise<number>;
export declare function closeDenoChannel(rid: number): Promise<string>;
export declare function checkDenoChannel(key: string): Promise<boolean>;
interface ChannelMessage {
    event: String;
    content: any;
}
interface Litype {
    name: String;
    fn: any;
}
export declare class DenoManager {
    constructor();
    static get(key: string): Promise<Deno | undefined>;
    static close(key: string): Promise<void>;
    static closeAll(): Promise<void>;
}
export declare class Deno extends Channel<ChannelMessage> {
    #private;
    arr: Litype[];
    static create(key: string): Promise<Deno>;
    constructor(key: string);
    get rid(): number;
    init(fn?: any): Promise<void>;
    send(name: string, value: any): Promise<string | null | undefined>;
    listenOn(name: string, fn: any): Promise<void>;
    unlistenFrom(name: string): Promise<void>;
    close(): Promise<void>;
}
export {};

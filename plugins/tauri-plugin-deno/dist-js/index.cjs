'use strict';

var core = require('@tauri-apps/api/core');

/******************************************************************************
Copyright (c) Microsoft Corporation.

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
***************************************************************************** */
/* global Reflect, Promise, SuppressedError, Symbol, Iterator */


function __classPrivateFieldGet(receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
}

function __classPrivateFieldSet(receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
}

typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
};

var _Deno_key, _Deno_rid, _Deno_status;
async function sendToDeno(value) {
    return await core.invoke("plugin:deno|send_to_deno", {
        ...value,
    });
}
async function listenOn(rid, name) {
    return await core.invoke("plugin:deno|listen_on", { rid, name }).then((r) => r);
}
async function unlistenFrom(rid, name) {
    return await core.invoke("plugin:deno|unlisten_from", { rid, name }).then((r) => (r.value ? r.value : null));
}
async function createDenoChannel(key, channel) {
    return await core.invoke("plugin:deno|create_deno_channel", {
        key: key,
        onEvent: channel,
    });
}
async function closeDenoChannel(rid) {
    return await core.invoke("plugin:deno|close_deno_channel", {
        payload: { rid },
    }).then((r) => r);
}
async function checkDenoChannel(key) {
    return await core.invoke("plugin:deno|check_deno_channel", {
        key: key,
    });
}
const manager = new Map();
let task = null;
class DenoManager {
    constructor() {
        if (!task) {
            task = setInterval(() => {
                //deno 健康检查
                for (let [key, _value] of manager.entries()) {
                    checkDenoChannel(key).then(re => {
                        if (!re) {
                            DenoManager.close(key);
                        }
                    });
                }
            }, 2000);
        }
    }
    static async get(key) {
        if (manager.has(key)) {
            return manager.get(key);
        }
        let deno = await Deno.create(key);
        if (!deno.rid) {
            console.log("deno instance undefined");
            return undefined;
        }
        manager.set(key, deno);
        return deno;
    }
    static async close(key) {
        let deno = manager.get(key);
        if (deno) {
            await deno.close();
            manager.delete(key);
        }
    }
    static async closeAll() {
        for (let [key, value] of manager.entries()) {
            await value.close();
            manager.delete(key);
        }
    }
}
//deno channe默认实现 主要用于后端的 deno服务的通信
class Deno extends core.Channel {
    static async create(key) {
        let deno = new Deno(key);
        await deno.init();
        return deno;
    }
    constructor(key) {
        super();
        _Deno_key.set(this, void 0);
        _Deno_rid.set(this, 0);
        _Deno_status.set(this, void 0);
        this.arr = [];
        __classPrivateFieldSet(this, _Deno_key, key, "f");
        __classPrivateFieldSet(this, _Deno_status, "start", "f");
        this.onmessage = (data) => {
            this.arr.forEach((item) => {
                if (item.name == data.event) {
                    item.fn(data.content);
                }
            });
        };
    }
    get rid() {
        return __classPrivateFieldGet(this, _Deno_rid, "f");
    }
    //初始化DenoChannel
    async init(fn) {
        if (__classPrivateFieldGet(this, _Deno_status, "f") == "start") {
            __classPrivateFieldSet(this, _Deno_rid, await createDenoChannel(__classPrivateFieldGet(this, _Deno_key, "f"), this), "f");
            __classPrivateFieldSet(this, _Deno_status, "run", "f");
            if (fn) {
                await fn();
            }
        }
    }
    //向deno发送消息
    async send(name, value) {
        if (__classPrivateFieldGet(this, _Deno_status, "f") == "close") {
            console.log("deno channel is closed");
            return;
        }
        return await sendToDeno({ rid: __classPrivateFieldGet(this, _Deno_rid, "f"), name, content: value });
    }
    //监听
    async listenOn(name, fn) {
        if (__classPrivateFieldGet(this, _Deno_status, "f") == "close") {
            console.log("deno channel is closed");
            return;
        }
        await listenOn(__classPrivateFieldGet(this, _Deno_rid, "f"), name);
        this.arr.push({ name, fn });
    }
    //解除监听
    async unlistenFrom(name) {
        if (__classPrivateFieldGet(this, _Deno_status, "f") == "close") {
            console.log("deno channel is closed");
            return;
        }
        await unlistenFrom(__classPrivateFieldGet(this, _Deno_rid, "f"), name);
    }
    //关闭
    async close() {
        await closeDenoChannel(__classPrivateFieldGet(this, _Deno_rid, "f"));
        __classPrivateFieldSet(this, _Deno_status, "close", "f");
    }
}
_Deno_key = new WeakMap(), _Deno_rid = new WeakMap(), _Deno_status = new WeakMap();

exports.Deno = Deno;
exports.DenoManager = DenoManager;
exports.checkDenoChannel = checkDenoChannel;
exports.closeDenoChannel = closeDenoChannel;
exports.createDenoChannel = createDenoChannel;
exports.listenOn = listenOn;
exports.sendToDeno = sendToDeno;
exports.unlistenFrom = unlistenFrom;

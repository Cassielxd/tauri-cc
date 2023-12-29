// deno-fmt-ignore-file
// deno-lint-ignore-file

// Copyright Joyent and Node contributors. All rights reserved. MIT license.
// Taken from Node 18.12.1
// This file is automatically generated by `tools/node_compat/setup.ts`. Do not modify this file manually.

'use strict';

const common = require('../common');
const EventEmitter = require('events');
const assert = require('assert');

const ee = new EventEmitter();
const handler = () => {};

assert.deepStrictEqual(ee.eventNames(), []);

assert.strictEqual(ee._events.hasOwnProperty, undefined);
assert.strictEqual(ee._events.toString, undefined);

ee.on('__proto__', handler);
ee.on('__defineGetter__', handler);
ee.on('toString', handler);

assert.deepStrictEqual(ee.eventNames(), [
  '__proto__',
  '__defineGetter__',
  'toString',
]);

assert.deepStrictEqual(ee.listeners('__proto__'), [handler]);
assert.deepStrictEqual(ee.listeners('__defineGetter__'), [handler]);
assert.deepStrictEqual(ee.listeners('toString'), [handler]);

ee.on('__proto__', common.mustCall(function(val) {
  assert.strictEqual(val, 1);
}));
ee.emit('__proto__', 1);

process.on('__proto__', common.mustCall(function(val) {
  assert.strictEqual(val, 1);
}));
process.emit('__proto__', 1);

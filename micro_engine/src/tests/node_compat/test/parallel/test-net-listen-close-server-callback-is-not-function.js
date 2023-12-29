// deno-fmt-ignore-file
// deno-lint-ignore-file

// Copyright Joyent and Node contributors. All rights reserved. MIT license.
// Taken from Node 18.12.1
// This file is automatically generated by `tools/node_compat/setup.ts`. Do not modify this file manually.

'use strict';
const common = require('../common');
const net = require('net');

const server = net.createServer(common.mustNotCall());

server.on('close', common.mustCall());

server.listen(0, common.mustNotCall());

server.close('bad argument');

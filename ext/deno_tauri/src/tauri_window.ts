const core = globalThis.Deno.core;
const ops = core.ops;
const primordials = globalThis.__bootstrap.primordials;
const {
  ArrayPrototypeFilter,
  Error,
  ObjectPrototypeIsPrototypeOf,
  String,
  StringPrototypeStartsWith,
  SymbolFor,
  SymbolIterator,
  SymbolToStringTag
} = primordials;
import * as webidl from "ext:deno_webidl/00_webidl.js";
import {
  defineEventHandler,
  ErrorEvent,
  EventTarget,
  MessageEvent,
  setIsTrusted
} from "ext:deno_web/02_event.js";

class TauriWindow extends EventTarget {
  #id = "";
  #status = "RUNNING";

  constructor(specifier: String, options = {}) {
    super();
  }

  #handleError(e: any) {


  }

  #pollControl = async () => {

  };
  #pollMessages = async () => {

  };
}
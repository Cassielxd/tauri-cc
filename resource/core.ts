//RequestMapping注解，类似于Java的springmvc
export function RequestMapping(method, path) {
  return function(
    target: Object,
    key: string | symbol,
    descriptor: PropertyDescriptor
  ) {
    const original = descriptor.value;

    function Fun(...args: any[]) {
      const result = original.apply(this, args);
      return result;
    }

    Fun.prototype.router = () => {
      return { method, path, key };
    };
    descriptor.value = Fun;
    return descriptor;
  };
}

//Controller注解，类似于springmvc
export function Controller(path: string) {
  return function(targetClass: any) {
    targetClass.prototype.path = () => path;
    return targetClass;
  };
}

//根据类构建Router
export function buildRouter(worker, className, Class) {
  const propertyNames = Object.getOwnPropertyNames(Class.prototype);
  const classDescriptor = Reflect.getOwnPropertyDescriptor(
    Class.prototype,
    "constructor"
  );

  const routers = {};
  if (classDescriptor.value.prototype && classDescriptor.value.prototype.path) {
    const path = classDescriptor.value.prototype.path;
    let p = path();
    if (p.startsWith("/")) {
      p = `/${worker}${p}`;
    } else {
      p = `/${worker}/${p}`;
    }
    for (const propertyName of propertyNames) {
      if (propertyName == "constructor") continue;
      const methodDescriptor = Reflect.getOwnPropertyDescriptor(
        Class.prototype,
        propertyName
      );
      if (
        methodDescriptor.value.prototype &&
        methodDescriptor.value.prototype.router
      ) {
        const router = methodDescriptor.value.prototype.router;
        const { method, path, key } = router();
        routers[p + path] = { className, method, key };
      }
    }
  }
  return routers;
}

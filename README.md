# StackParam

StackParam is a utility that gives method parameters to Java 8 stack traces. It is written in Rust and built to be
fairly unobtrusive.

It adds the parameter information to stack trace outputs and can be used to programmatically obtain method parameters
(including "this" for non-static methods) up the stack.

## Quick Start

StackParam is a shared library loaded as a [JVMTI](http://docs.oracle.com/javase/8/docs/platform/jvmti/jvmti.html)
agent (see [Installation](#installation) to get it). For example, say the following Java class is at
`HelloFailure.java`:

```java
public class HelloFailure {
    public static void main(String[] args) {
        throwException(42);
    }

    public static void throwException(int foo) {
        throw new RuntimeException("Hello, Failure!");
    }
}
```

Compile with `javac ./HelloFailure.java` which will create the `HelloFailure.class` file. Now run via Java while
specifying the shared library path:

    java -agentpath:path/to/shared.ext HelloFailure --some-arg --another-arg 10

The file name `shared.ext` might be `stackparam.dll` on Windows, `libstackparam.so` on Linux, etc. The output is:

    Exception in thread "main" java.lang.RuntimeException: Hello, Failure!
            at HelloFailure.throwException(HelloFailure.java:0) [arg0=42]
            at HelloFailure.main(HelloFailure.java:0) [arg0=[--some-arg, --another-arg, 10]]

Compiling with the `-g` option passed to `javac` gives debug information to StackParam so the parameter names are
accurate. The output for the same code as above compiled with debug info is:

    Exception in thread "main" java.lang.RuntimeException: Hello, Failure!
            at HelloFailure.throwException(HelloFailure.java:0) [foo=42]
            at HelloFailure.main(HelloFailure.java:0) [args=[--some-arg, --another-arg, 10]]

Note that StackParam works with all exception stack trace uses and even provides a mechanism to programmatically obtain
method parameters up the call stack.

## Installation

if you are using 64-bit Windows or Linux, the easiest way is to download the latest stackparam.dll or libstackparam.so
from the releases area. For Mac or other architectures, I don't have a precompiled shared lib but it should be easy to
build. See [Manually Building](#manually-building).

### Java Versions

This should work with OpenJDK/Oracle 8. It might also work on OpenJDK/Oracle 7 if manually compiled as the injection
points are similar, but this is untested. This will not yet work with OpenJDK/Oracle <= 6 or OpenJDK/Oracle 9. It will
also not work on other JREs whose runtimes are not based on the OpenJDK stdlib.

It simply didn't suit my needs to support other JVMs (yet), but it would be fairly trivial to implement if enough people
want it.

I doubt Android would be as straightforward. [This](https://code.google.com/p/android/issues/detail?id=60961) seems to
imply there is no JVMTI interface. If that was in place, the
[`Throwable`](https://android.googlesource.com/platform/libcore/+/6975f84c2ed72e1e26d20190b6f318718c849008/ojluni/src/main/java/java/lang/Throwable.java)
class there would need the specifically targeted injections to support our needs which is no big deal.

### Manually Building

This library is written in Rust and compiles a Java class at build time. Therefore the prerequisites are a recent
installation of [Rust](https://www.rust-lang.org/) and a JDK 8 installation with `javac` on the `PATH`.

Once the prerequisites are installed, the tests can be run via `cargo`:

    cargo test

This does several things internally including running Gradle (automatically downloaded if not present) to build a Java
class and running Gradle again to do some Java-side tests.

If the tests succeed, build can be done via `cargo` as well:

    cargo build --release

Once built, the shared library is in `target/release/shared.ext` where `shared.ext` might be `stackparam.dll` on
Windows, `libstackparam.so` on Linux, etc.

## Usage

Once the agent is loaded, it automatically injects strings into stack traces.

### Loading the Agent

While [JVMTI](http://docs.oracle.com/javase/8/docs/platform/jvmti/jvmti.html#deployingAgents) has a few approaches to
deploying an agent, this agent needs to be deployed via command line because it needs to hook into the very earliest
part of the JVM load. This is easily done via the `-agentpath` path parameter of the `java` command, e.g.:

    java -agentpath:path/to/shared.ext HelloWorld

Instead of `-agentpath` for the exact path, `-agentlib` could be used to just give the library name. Assuming the
`stackparam.dll` is on the `PATH` in Windows or `libstackparam.so` is in the shared library location (e.g. an
overridden `LD_LIBRARY_PATH`) in Linux, you can run:

    java -agentlib:stackparam HelloWorld

Note, although untested, this library can likely be placed in the JRE's `lib/amd64` folder to get the same effect.

### Logging

This library uses Rust's [env_logger](https://doc.rust-lang.org/log/env_logger/) which lets the logging be controlled
by the `RUST_LOG` environment variable. The binary name is `stackparam`, so setting `RUST_LOG` to `stackparam=info`
shows info logs, `stackparam=debug` shows debug logs, and just `stackparam` shows all logs. The logs are emitted to the
standard error stream.

### Programmatic Value Access

In addition to just showing parameters, the
[`stackparam.StackParamNative`](javalib/native/src/main/java/stackparam/StackParamNative.java) class is automatically injected.
This class provides the following `loadStackParams` method:

```java
/**
 * Returns the stack params of the given thread for the given depth. It is
 * returned with closest depth first.
 *
 * Each returned sub array (representing a single depth) has params
 * including "this" as the first param for non-static methods. Each param
 * takes 3 values in the array: the string name, the string JVM type
 * signature, and the actual object. All primitives are boxed.
 *
 * In cases where the param cannot be obtained (i.e. non-"this" for native
 * methods), the string "<unknown>" becomes the value regardless of the
 * type's signature.
 *
 * @param thread The thread to get params for
 * @param maxDepth The maximum depth to go to
 * @return Array where each value represents params for a frame. Each param
 *         takes 3 spots in the sub-array for name, type, and value.
 * @throws NullPointerException If thread is null
 * @throws IllegalArgumentException If maxDepth is negative
 * @throws RuntimeException Any internal error we were not prepared for
 */
public static native Object[][] loadStackParams(Thread thread, int maxDepth);
```

Since this is automatically injected, it can easily be accessed via reflection. The problem with accessing via
reflection is there are a few stack frames between the caller and the reflected method that `invoke` is called on which
makes the params less predictably navigable.

Instead, you can either add the above contents in a `stackparam.StackParamNative` class in your source code or you can
add a dependency to the `native` library JAR to your build file via
[`JitPack`](https://jitpack.io/#com.github.cretz.stackparam/native/0.1.0). Even though it is included, the file/JAR
will never be directly used because the actual class is injected early at VM startup.

Once in place, you can do neat things like grab the caller, e.g.:

```java
public class CallerTest {
    /** Check that this is only called via instance method of foo.Bar */
    public boolean isFooBarInstanceMethod() {
        // We have to give a max depth of 3 and access the third one because on
        // the stack, "loadStackParams" is at 0, this method ("checkCaller") is
        // at 1, and then the caller is at 2.
        Object[] callerParams = stackparam.StackParamNative.loadStackParams(Thread.currentThread(), 3)[2];
        return callerParams.length != 0 &&
            "this".equals(callerParams[0]) &&
            callerParams[2] instanceof foo.Bar;
    }
}
```

While this kind of programming/validation in general is very bad and not portable, it demonstrates usage of the library.
Also, it is very high performing.

There is also a `public static String appendParamsToFrameString(String frameString, Object[] params)` method on the
class which takes the given set of `params` triplets and appends it (after a space) to the given `frameString` and
returns it. It is mostly a helper for the library, but can be used by others.

### Production Usage?

I wouldn't, but I took care to silently fail and fall back to original JVM functionality in most cases. There are a few
possible burdensome performance issues:

* Stack walking at exception creation (CPU) - This is not extremely heavy and note that the JVM does its own native
  stack walking at this time to build the stack trace in the first place. Usually this is not an issue except for those
  using exceptions for control flow, and in those cases the developers should be overriding `fillInStackTrace()` to do
  nothing which will also make this library do nothing.
* Holding object references (memory) - For every exception stack trace created, a multi-dimensional array is created to
  hold references to all local params, their names, and their signatures. Usually once an exception is thrown, many of
  the local variables on the stack go out of scope and are eligible for garbage collection. With StackParam, the
  references (and the strings) live at least as long as the exception does. Besides just the multi-dimensional array on
  `Throwable`, a reference is also placed on `StackTraceElement` to the individual array item.
* According to
  [this StackOverflow question](http://stackoverflow.com/questions/24108591/overhead-of-enabling-jvmti-capability-to-query-local-variables),
  since we tell the JVM we want local variables, a couple of optimizations might be disabled. I have not independently
  verified this.

There are also a couple of security concerns:

* Low-level library - Care should always be taken when adding native code behind the JVM in production. There are no
  guarantees of the safety of the software. Granted, the danger surface area is not much higher than untrusted Java
  code.
* Sensitive data - StackParam does not know what is considered confidential data and what is not. Data/method filtering
  is not yet implemented. So your parameters to `BCrypt.checkpw` for example would be visible to all if an exception
  occurred inside it.

## How Does it Work?

StackParam takes a series of steps to do what it does. In no particular order, they are:

* During Rust build time, compile `stackparam.StackParamNative` to a class file and include the bytes into the shared
  library.
* On agent start, ask the JVM to access local vars and class file load events. Also register callbacks for VM init and
  class file load hook.
* On VM init, take the `stackparam.StackParamNative` bytes and inject the class via
  [`DefineClass`](http://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#DefineClass).
* Just before `Throwable` class load, transform the class bytes to:
  * Add a `private transient Object[][] stackParams` field to the class.
  * Add a `private native Throwable stackParamFillInStackTrace(Thread)` method to the class.
  * Change the existing `fillInStackTrace` method to find the internal native `fillStackTrace` overload call. Then
    inject instructions to call our `stackParamFillInStackTrace(Thread)` method afterwards.
  * Rename the existing `getOurStackTrace` method to `$$stack_param$$getOurStackTrace`.
  * Add a `private synchronized native StackTraceElement[] getOurStackTrace` method to the class.
* Just before `StackTraceElement` class load, transform the class bytes to:
  * Add a `transient Object[] paramInfo` field to the class.
  * Rename the existing `toString` method to `$$stack_param$$toString`.
  * Add a `public native String toString` method to the class.
* On native invoke of `stackparam.StackParamNative.loadStackParams`, walk up the stack grabbing params and return them.
* On the native invoke of `Throwable.stackParamFillInStackTrace`:
  * Call `getStackTraceDepth` to fetch the depth of the current stack trace.
  * Walk up the stack grabbing params for only the stack trace depth (plus a little for ourselves).
  * Take the last depth amount of params, and store in `Throwable.stackParams`.
* On the native invoke of `Throwable.getOurStackTrace`:
  * Record state of `Throwable.stackTrace`.
  * Call `Throwable.$$stack_param$$getOurStackTrace` for the array of `StackTraceElement` values.
  * Bail if the result isn't different than the recorded field value.
  * Take the resulting array of `StackTraceElement` values and set each one's `paramInfo` field to its set of params
    from the `Throwable.stackParams` field.
* On the native invoke of `StackTraceElement.toString`:
  * Call `StackTraceElement.$$stack_param$$toString`.
  * Run the result through `stackparam.StackParamNative.appendParamsToFrameString` method along with the
    `StackTraceElement.paramInfo` value to return a string with parameters.

While it looks like a good bit of reverse engineering and a bit brittle, it's not that bad. Failures are handled
gracefully for the most part. And it is not that difficult to add conditionals and do different bytecode manipulation
based on what is seen (e.g. for different Java versions or different JVMs).

## Why?

Primary goals:
* Learn Rust (only kinda, I live in unsafe code which isn't cool)
* Learn Rust + JNI/JVMTI
* Learn intricacies of JVMTI and more about JVM internals
* Provide nice example project for others wanting to hook into the JVM

Secondary goals:
* Actually get method params on the stack trace

## TODO

* Other JVM versions
  * Especially Java 9, where we can just have params on
    [StackFrame](http://download.java.net/java/jdk9/docs/api/java/lang/StackWalker.StackFrame.html) and the callers can
    choose how they walk it.
* Proper ignoring of certain OOM exceptions, see
  [this](http://hg.openjdk.java.net/jdk8/jdk8/hotspot/file/87ee5ee27509/src/share/vm/memory/universe.cpp#l557) for some
  special exceptions that don't get traces.
* Options such as filtering, disabling, etc.
* Stop checking for JNI errors on every invocation, but only on error situations like null responses.

## Acknowledgements

The entire [bytecode manipulation](src/bytecode) library was taken from https://github.com/xea/rust-jvmti with many
thanks. I also looked at that repo quite a bit when I was getting started and took the initial JVMTI definitions from
there.

Also, this project uses the JNI definitions from https://github.com/sfackler/rust-jni-sys.
package stackparam;

public class StackParamNative {

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
     */
    public static native Object[][] loadStackParams(Thread thread, int maxDepth);
}

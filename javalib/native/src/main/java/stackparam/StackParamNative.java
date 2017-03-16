package stackparam;

public class StackParamNative {

    /**
     * Returns the stack params of the given thread for the given depth. It is
     * returned with closest depth first.
     *
     * Each returned sub array (representing a single depth) has locals
     * including "this" as the first local for non-static ones. Each local
     * takes 3 values in the array: the string name, the string JVM type
     * signature, and the actual object. In cases where the value cannot be
     * obtained, the string "<unknown>" becomes the value regardless of the
     * type's signature. All primitives are boxed.
     *
     * Depths without any locals (or are excluded for whatever reason) are
     * represented by a null array. The result of this method will never be a
     * null array.
     */
    public static native Object[][] loadStackParams(Thread thread, int maxDepth);
}

package stackparam;

import static org.junit.Assert.*;
import org.junit.Test;

import java.util.Arrays;

public class StackParamNativeTest {

    private static final Object CONST_OBJ = new Object();

    @Test
    public void testLoadStackParamsSimple() {
        Object[][] stackParams = instanceWithStringArg("Some string");

        Object[] expectedOtherMethodArgs = {
            "boolArg", "Z", true,
            "byteArg", "B", (byte) 100,
            "charArg", "C", (char) 101,
            "shortArg", "S", (short) 102,
            "intArg", "I", 103,
            "longArg", "J", 104L,
            "floatArg", "F", 105.6f,
            "doubleArg", "D", 106.7,
            "nullArg", "Ljava/lang/Object;", null,
            "objectExactArg", "Ljava/lang/Object;", CONST_OBJ,
            "stringVarArgs", "[Ljava/lang/String;", new String[] { "foo", "bar", "baz" }
        };
        assertArrayEquals(expectedOtherMethodArgs, stackParams[0]);

        Object[] expectedStringMethodArgs = {
            "this",
            "L" + getClass().getName().replace('.', '/') + ";",
            this,
            "stringArg",
            "Ljava/lang/String;",
            "Some string"
        };
        assertArrayEquals(expectedStringMethodArgs, stackParams[1]);
    }

    private Object[][] instanceWithStringArg(String stringArg) {
        return withOtherArgs(true, (byte) 100, (char) 101,
                (short) 102, 103, 104L,
                105.6f, 106.7, null,
                CONST_OBJ, "foo", "bar", "baz");
    }

    private static Object[][] withOtherArgs(boolean boolArg, byte byteArg, char charArg,
                                            short shortArg, int intArg, long longArg,
                                            float floatArg, double doubleArg, Object nullArg,
                                            Object objectExactArg, String... stringVarArgs) {
        Object[][] stackParams;
        try {
            stackParams = (Object[][]) Class.forName("stackparam.StackParamNative").
                    getMethod("loadStackParams", Thread.class, int.class).
                    invoke(null, Thread.currentThread(), 500);
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
        // Grab the exception stack trace, and use the stack element count to
        // know how much to trim off the top of our params. The problem is that
        // the stack we're given includes the reflection call which has a few
        // classes.
        return Arrays.copyOfRange(stackParams,
                stackParams.length - (new Exception().getStackTrace().length), stackParams.length);
    }
}

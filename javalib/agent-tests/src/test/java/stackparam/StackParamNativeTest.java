package stackparam;

import org.junit.Test;

import static org.junit.Assert.assertArrayEquals;

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
        assertArrayEquals(expectedOtherMethodArgs, stackParams[1]);

        Object[] expectedStringMethodArgs = {
            "this",
            "L" + getClass().getName().replace('.', '/') + ";",
            this,
            "stringArg",
            "Ljava/lang/String;",
            "Some string"
        };
        assertArrayEquals(expectedStringMethodArgs, stackParams[2]);
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
        return StackParamNative.loadStackParams(Thread.currentThread(), 3);
    }
}

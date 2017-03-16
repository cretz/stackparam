package stackparam;

import static org.junit.Assert.*;
import org.junit.Test;

import java.util.Arrays;

public class StackParamNativeTest {

    @Test
    public void testLoadStackParamsSimple() {
        Object[][] stackParams = instanceWithStringArg("Some string");

        Object[] expectedLongMethodArgs = {
            "longArg",
            "J",
            Integer.MAX_VALUE + 5L
        };
        assertArrayEquals(expectedLongMethodArgs, stackParams[0]);

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
        return withLongArg(Integer.MAX_VALUE + 5L);
    }

    private static Object[][] withLongArg(long longArg) {
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

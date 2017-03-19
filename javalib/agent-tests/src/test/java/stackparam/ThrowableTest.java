package stackparam;

import static org.junit.Assert.*;
import org.junit.Test;

public class ThrowableTest {

    private static final Object CONST_OBJ = new Object();

    @Test
    public void testStackTraceElementToString() throws Exception {
        String expected = "[this=" + this + ", boolArg=true, byteArg=100, " +
                "charArg=e, shortArg=102, intArg=103, longArg=104, " +
                "floatArg=105.6, doubleArg=106.7, nullArg=null, " +
                "objectExactArg=" + CONST_OBJ + ", stringVarArgs=[foo, bar, baz]]";
        StackTraceElement elem = getTestElement();
        String traceStr = elem.toString();
        traceStr = traceStr.substring(traceStr.indexOf('['));
        assertEquals(expected, traceStr);
    }

    private StackTraceElement getTestElement() {
        try {
            methodThatWillThrow(true, (byte) 100, (char) 101,
                    (short) 102, 103, 104L,
                    105.6f, 106.7, null,
                    CONST_OBJ, "foo", "bar", "baz");
            fail();
            return null;
        } catch (RuntimeException e) {
            return e.getStackTrace()[0];
        }
    }

    private void methodThatWillThrow(boolean boolArg, byte byteArg, char charArg,
                                                 short shortArg, int intArg, long longArg,
                                                 float floatArg, double doubleArg, Object nullArg,
                                                 Object objectExactArg, String... stringVarArgs) {
        throw new RuntimeException("OH!");
    }
}

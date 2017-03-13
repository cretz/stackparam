package stackparam;

import org.junit.Test;

import java.lang.reflect.Method;

public class HelloWorldTest {

    @Test
    public void testSimple() throws Exception {
        System.out.println("Hello, world");
        for (Method method : Class.forName("stackparam.Ow2AsmManip").getDeclaredMethods()) {
            System.out.println("METHOD: " + method.toString());
        }
    }
}

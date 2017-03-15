package stackparam;

import org.junit.Assert;
import org.junit.Test;

public class HelloWorldTest {

    @Test
    public void testSimple() throws Exception {
        Assert.assertEquals("Awesome!!", Throwable.class.getMethod("testSomething").invoke(null));
    }
}

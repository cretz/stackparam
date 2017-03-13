package stackparam;

import com.google.common.io.ByteStreams;
import jdk.internal.org.objectweb.asm.ClassReader;
import jdk.internal.org.objectweb.asm.tree.ClassNode;
import jdk.internal.org.objectweb.asm.tree.MethodNode;
import org.junit.Assert;
import org.junit.Test;

import java.io.InputStream;

public class Ow2AsmManipTest {

    @Test
    public void testMethodIsAdded() throws Exception {
        byte[] classBytes;
        try (InputStream is = Ow2AsmManip.class.getResourceAsStream("Ow2AsmManip.class")) {
            classBytes = ByteStreams.toByteArray(is);
        }
        classBytes = Ow2AsmManip.addThrowableMethod(classBytes);
        ClassNode node = new ClassNode();
        new ClassReader(classBytes).accept(node, 0);
        MethodNode method = node.methods.stream().filter(m -> "testCall".equals(m.name)).findFirst().get();
        Assert.assertEquals(1, method.maxStack);
    }
}

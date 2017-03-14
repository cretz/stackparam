package stackparam;

import jdk.internal.org.objectweb.asm.ClassReader;
import jdk.internal.org.objectweb.asm.ClassWriter;
import jdk.internal.org.objectweb.asm.Opcodes;
import jdk.internal.org.objectweb.asm.Type;
import jdk.internal.org.objectweb.asm.tree.ClassNode;
import jdk.internal.org.objectweb.asm.tree.InsnNode;
import jdk.internal.org.objectweb.asm.tree.LdcInsnNode;
import jdk.internal.org.objectweb.asm.tree.MethodNode;

import java.util.Collections;

class Ow2AsmManip {
    public void testSomethingElse() {

    }

    public static String testSomething() {
        return "Test!!2";
    }

    public static byte[] addThrowableMethod(byte[] throwableClassBytes) {
        System.out.println("JVM SIDE, in byte size: " + throwableClassBytes.length);
        ClassNode node = new ClassNode();
        new ClassReader(throwableClassBytes).accept(node, 0);

        // Only if method doesn't exist
        boolean exists = false;
        for (MethodNode method : node.methods) {
            if ("testCall".equals(method.name)) {
                return throwableClassBytes;
            }
        }

        MethodNode method = new MethodNode();
        method.access = Opcodes.ACC_PUBLIC + Opcodes.ACC_STATIC;
//        method.name = "getStackParams";
//        method.desc = Type.getMethodDescriptor(Type.getType(Object[][].class), Type.BOOLEAN_TYPE);
        method.exceptions = Collections.emptyList();
        method.name = "testCall";
        method.desc = Type.getMethodDescriptor(Type.getType(String.class));
        method.instructions.add(new LdcInsnNode("Awesome!"));
        method.instructions.add(new InsnNode(Opcodes.ARETURN));
        node.methods.add(method);

        ClassWriter writer = new ClassWriter(ClassWriter.COMPUTE_FRAMES + ClassWriter.COMPUTE_MAXS);
        node.accept(writer);
        byte[] ret = writer.toByteArray();
        System.out.println("JVM SIDE, out byte size: " + ret.length);
        return ret;
    }
}

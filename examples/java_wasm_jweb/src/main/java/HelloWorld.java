import de.inetsoftware.jwebassembly.api.annotation.Export;
public class HelloWorld {

    @Export
    public static void main() {

    }

    @Export
   public static Integer add_one(Integer a) {
       return a + 1;
   }

}
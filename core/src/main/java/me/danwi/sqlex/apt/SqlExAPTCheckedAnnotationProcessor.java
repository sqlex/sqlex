package me.danwi.sqlex.apt;

import me.danwi.sqlex.core.annotation.repository.SqlExAPTChecked;

import javax.annotation.processing.AbstractProcessor;
import javax.annotation.processing.RoundEnvironment;
import javax.lang.model.SourceVersion;
import javax.lang.model.element.Element;
import javax.lang.model.element.PackageElement;
import javax.lang.model.element.TypeElement;
import javax.tools.JavaFileObject;
import java.io.Writer;
import java.util.HashSet;
import java.util.Set;

public class SqlExAPTCheckedAnnotationProcessor extends AbstractProcessor {
    public static String CheckedStubClassName = "IfThisClassUndefinedPleaseCheckAPTConfig";

    @Override
    public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment roundEnv) {
        Set<? extends Element> repositoryClasses = roundEnv.getElementsAnnotatedWith(SqlExAPTChecked.class);
        for (Element repositoryClass : repositoryClasses) {
            if (repositoryClass instanceof TypeElement)
                generateCheckStub((TypeElement) repositoryClass);
        }
        return false;
    }

    private void generateCheckStub(TypeElement repositoryClass) {
        Element enclosingElement = repositoryClass.getEnclosingElement();
        if (!(enclosingElement instanceof PackageElement))
            return;
        //根包
        String rootPackage = ((PackageElement) enclosingElement).getQualifiedName().toString();
        try {
            JavaFileObject sourceFile = processingEnv.getFiler().createSourceFile(rootPackage + "." + CheckedStubClassName);
            try (Writer writer = sourceFile.openWriter()) {
                writer.write("package " + rootPackage + ";\n\n" + "class " + CheckedStubClassName + "{}");
            }
        } catch (Exception ignored) {
        }
    }

    @Override
    public Set<String> getSupportedAnnotationTypes() {
        Set<String> annotations = new HashSet<>();
        annotations.add(SqlExAPTChecked.class.getName());
        return annotations;
    }

    @Override
    public SourceVersion getSupportedSourceVersion() {
        return SourceVersion.latestSupported();
    }
}

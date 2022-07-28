package me.danwi.sqlex.apt;

import me.danwi.sqlex.core.annotation.repository.SqlExConverterCheck;

import javax.annotation.processing.AbstractProcessor;
import javax.annotation.processing.Messager;
import javax.annotation.processing.ProcessingEnvironment;
import javax.annotation.processing.RoundEnvironment;
import javax.lang.model.SourceVersion;
import javax.lang.model.element.*;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import javax.tools.Diagnostic;
import java.util.*;

public class SqlExConverterCheckAnnotationProcessor extends AbstractProcessor {
    private SqlExProcessorUtil util;
    private Types typesUtil;
    private Elements elementsUtil;
    private Messager messager;

    @Override
    public synchronized void init(ProcessingEnvironment processingEnv) {
        util = new SqlExProcessorUtil(processingEnv);
        typesUtil = processingEnv.getTypeUtils();
        elementsUtil = processingEnv.getElementUtils();
        messager = processingEnv.getMessager();
        super.init(processingEnv);
    }

    @Override
    public Set<String> getSupportedAnnotationTypes() {
        Set<String> annotations = new HashSet<>();
        annotations.add(SqlExConverterCheck.class.getName());
        return annotations;
    }

    @Override
    public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment roundEnv) {
        Set<? extends Element> repositoryElements = roundEnv.getElementsAnnotatedWith(SqlExConverterCheck.class);
        for (Element element : repositoryElements) {
            if (element instanceof TypeElement)
                checkRepositoryInterface((TypeElement) element);
        }
        return true;
    }

    private void checkRepositoryInterface(TypeElement repositoryTypeElement) {
        try {
            //获取到接口上的所有转换器
            List<TypeElement> allConverterElements = util.getAllParameterConverterElements(repositoryTypeElement);
            //判断其是否合法
            for (TypeElement converterElement : allConverterElements) {
                try {
                    util.isValidateParameterConverter(converterElement);
                } catch (Exception e) {
                    messager.printMessage(Diagnostic.Kind.ERROR, util.getQualifiedName(converterElement) + ": " + e.getMessage());
                }
            }
        } catch (Exception e) {
            messager.printMessage(Diagnostic.Kind.ERROR, util.getQualifiedName(repositoryTypeElement) + ": " + e.getMessage());
        }
    }


    @Override
    public SourceVersion getSupportedSourceVersion() {
        return SourceVersion.latestSupported();
    }
}

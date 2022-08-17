package me.danwi.sqlex.apt;

import me.danwi.sqlex.common.Paged;
import me.danwi.sqlex.core.annotation.method.SqlExPaged;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExParameterCheck;
import me.danwi.sqlex.core.annotation.SqlExRepository;

import javax.annotation.processing.AbstractProcessor;
import javax.annotation.processing.Messager;
import javax.annotation.processing.ProcessingEnvironment;
import javax.annotation.processing.RoundEnvironment;
import javax.lang.model.SourceVersion;
import javax.lang.model.element.*;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.MirroredTypeException;
import javax.lang.model.type.TypeKind;
import javax.lang.model.type.TypeMirror;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import javax.tools.Diagnostic;
import java.util.*;

public class SqlExParameterCheckAnnotationProcessor extends AbstractProcessor {
    private SqlExProcessorUtil util;
    private Types typesUtil;
    private Elements elementsUtil;
    private Messager messager;

    //转换器配置
    private final Map<String, List<String>> converters = new HashMap<>();

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
        annotations.add(SqlExParameterCheck.class.getName());
        return annotations;
    }

    @Override
    public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment roundEnv) {
        Set<? extends Element> methods = roundEnv.getElementsAnnotatedWith(SqlExParameterCheck.class);
        for (Element method : methods) {
            try {
                if (method instanceof ExecutableElement)
                    checkMethod((ExecutableElement) method);
            } catch (Exception e) {
                messager.printMessage(Diagnostic.Kind.ERROR, util.getQualifiedName(method) + ": " + e.getMessage());
            }
        }
        return true;
    }

    //获取repository上参数类型转换器所对应的from type
    private synchronized List<String> getRegisteredTypes(TypeMirror repositoryType) throws Exception {
        List<String> registeredTypes = converters.get(util.getQualifiedName(repositoryType));
        //判断缓存是否合法
        if (registeredTypes == null) {
            //获取到这个repository上所有的注册的转换器
            Element repositoryElement = typesUtil.asElement(repositoryType);
            if (!(repositoryElement instanceof TypeElement))
                throw new Exception("该方法所在的Repository接口异常");
            List<TypeElement> allParameterConverterElements = util.getAllParameterConverterElements((TypeElement) repositoryElement);
            //获取到所有转换器的来源类型
            List<String> fromTypes = new ArrayList<>();
            for (TypeElement parameterConverterElement : allParameterConverterElements) {
                TypeMirror fromType = util.isValidateParameterConverter(parameterConverterElement);
                fromTypes.add(util.getQualifiedName(fromType));
            }
            converters.put(util.getQualifiedName(repositoryType), fromTypes);
            registeredTypes = fromTypes;

        }
        return registeredTypes;
    }

    //检查该方法下的所有参数是否合法
    private void checkMethod(ExecutableElement method) throws Exception {
        //获取该方法所在的接口
        Element enclosingElement = method.getEnclosingElement();
        if (!(enclosingElement instanceof TypeElement))
            throw new Exception("该方法不在一个接口中");
        TypeElement interfaceElement = (TypeElement) enclosingElement;
        //获取其repository接口
        SqlExRepository repositoryAnnotation = interfaceElement.getAnnotation(SqlExRepository.class);
        TypeMirror repositoryType = null;
        try {
            //noinspection ResultOfMethodCallIgnored
            repositoryAnnotation.value();
        } catch (MirroredTypeException e) {
            repositoryType = e.getTypeMirror();
        }
        if (repositoryType == null)
            throw new Exception("无法获取该方法所在的repository");
        //获取能够被转换到数据类型
        List<String> registeredTypes = Collections.emptyList();
        try {
            registeredTypes = getRegisteredTypes(repositoryType);
        } catch (Exception ignored) {
        }
        //是否是一个分页方法
        boolean isPagedMethod = method.getAnnotation(SqlExPaged.class) != null;
        //获取所有的参数,挨个检查
        for (VariableElement parameter : method.getParameters()) {
            TypeMirror parameterType = parameter.asType();
            //先检查分页参数
            if (isPagedMethod) {
                Name parameterName = parameter.getSimpleName();
                if (parameterName.contentEquals(Paged.PageSizeParameterName)) {
                    if (parameterType.getKind() != TypeKind.LONG)
                        throw new Exception("pageSize参数的类型必须为long");
                    continue;
                }
                if (parameterName.contentEquals(Paged.PageNoParameterName)) {
                    if (parameterType.getKind() != TypeKind.LONG)
                        throw new Exception("pageNo参数的类型必须为long");
                    continue;
                }
            }
            //普通参数,只需要判断数据类型
            //判断他是否为一个List
            if (util.getQualifiedName(parameterType).contentEquals(List.class.getName())) {
                parameterType = ((DeclaredType) parameterType).getTypeArguments().get(0);
            }
            if (!util.isSupportedType(typesUtil.asElement(parameterType), registeredTypes)) {
                throw new Exception("参数 " + parameter.getSimpleName() + " 使用了不被支持的参数数据类型(" + util.getQualifiedName(parameterType) + ")");
            }
        }
    }

    @Override
    public SourceVersion getSupportedSourceVersion() {
        return SourceVersion.latestSupported();
    }
}

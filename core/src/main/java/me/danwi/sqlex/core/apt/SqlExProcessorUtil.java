package me.danwi.sqlex.core.apt;

import me.danwi.sqlex.common.ParameterTypes;
import me.danwi.sqlex.core.annotation.SqlExConverter;

import javax.annotation.processing.ProcessingEnvironment;
import javax.lang.model.element.*;
import javax.lang.model.type.MirroredTypeException;
import javax.lang.model.type.TypeMirror;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;

public class SqlExProcessorUtil {
    private final ProcessingEnvironment env;
    private final Types types;
    private final Elements elements;

    public SqlExProcessorUtil(ProcessingEnvironment env) {
        this.env = env;
        this.types = env.getTypeUtils();
        this.elements = env.getElementUtils();
    }

    public String getQualifiedName(TypeMirror type) {
        return getQualifiedName(types.asElement(type));
    }

    public String getQualifiedName(Element element) {
        if (element instanceof QualifiedNameable) {
            return ((QualifiedNameable) element).getQualifiedName().toString();
        }
        return element.toString();
    }

    public Element getSuperClass(TypeMirror type) {
        return getSuperClass(types.asElement(type));
    }

    public Element getSuperClass(Element element) {
        List<? extends TypeMirror> supertypes = this.types.directSupertypes(element.asType());
        if (supertypes.isEmpty())
            return null;
        return this.types.asElement(supertypes.get(0));
    }

    //获取类/接口上的所有通过注解声明的参数转换器类型
    public List<TypeElement> getAllParameterConverterElements(TypeElement element) throws Exception {
        ArrayList<TypeElement> result = new ArrayList<>();
        SqlExConverter[] converters = element.getAnnotationsByType(SqlExConverter.class);
        for (SqlExConverter converter : converters) {
            TypeMirror typeMirror = null;
            try {
                //noinspection ResultOfMethodCallIgnored
                converter.converter();
            } catch (MirroredTypeException e) {
                typeMirror = e.getTypeMirror();
            }
            if (typeMirror == null)
                throw new Exception("编译时无法获取参数转化器信息");
            //转换成element
            Element converterElement = types.asElement(typeMirror);
            if (converterElement == null)
                throw new Exception("编译时无法获取参数转换器的类型信息");
            if (converterElement instanceof TypeElement)
                result.add((TypeElement) converterElement);
        }
        return result;
    }

    //判断这个转换器是不是一个合法的参数类型转换器,如果是,则返回他的from type
    public TypeMirror isValidateParameterConverter(TypeElement element) throws Exception {
        //判断他是否实现了ParameterConverter接口
        boolean isConverter = element.getInterfaces().stream()
                .map(types::asElement)
                .anyMatch(it -> {
                    if (it instanceof TypeElement) {
                        return ((TypeElement) it).getQualifiedName().contentEquals(ParameterTypes.ParameterConverterInterfaceQualifiedName);
                    }
                    return false;
                });
        if (!isConverter) {
            throw new Exception("未实现参数类型转换器接口");
        }
        //获取所有的构造函数,判断其是否有无参构造函数
        if (element.getEnclosedElements().stream()
                .filter(it -> it.getKind() == ElementKind.CONSTRUCTOR)
                .map(it -> {
                    if (it instanceof ExecutableElement)
                        return (ExecutableElement) it;
                    return null;
                })
                .filter(Objects::nonNull)
                .noneMatch(it -> it.getParameters().size() == 0))
            throw new Exception("参数类型转换器必须包含一个无参构造函数");
        //获取到convert方法,从而判断其类型是否正确
        List<? extends Element> methods = element.getEnclosedElements().stream()
                .filter(it -> it.getKind() == ElementKind.METHOD && it.getSimpleName().contentEquals("convert")) //名称限定
                .map(it -> {
                    if (it instanceof ExecutableElement)
                        return (ExecutableElement) it;
                    return null;
                })
                .filter(Objects::nonNull)
                .filter(it -> it.getParameters().size() == 1) //参数个数限定
                .collect(Collectors.toList());
        if (methods.size() != 1) {
            throw new Exception("参数类型转换器中有且只能有一个名为 convert 的方法");
        }
        //转换方法
        ExecutableElement convertMethod = (ExecutableElement) methods.get(0);
        //获取其返回类型
        TypeMirror toType = convertMethod.getReturnType();
        //判断其是否是预支持类型
        if (!isPreSupportedType(types.asElement(toType))) {
            throw new Exception("参数类型转换器的目标类型 " + getQualifiedName(toType) + " 不是预支持的数据类型");
        }
        //获取其来源类型
        TypeMirror typeMirror = convertMethod.getParameters().get(0).asType();
        if (typeMirror == null)
            throw new Exception("无法确定参数类型转换器的目标类型");
        return typeMirror;
    }


    //判断这个类型是否为预支持类型
    private boolean isPreSupportedType(Element element) {
        String qualifiedName = getQualifiedName(element);
        if (Arrays.asList(ParameterTypes.PreSupportedTypes).contains(qualifiedName)) {
            return true;
        }
        Element superClass = getSuperClass(element);
        if (superClass == null)
            return false;
        return isPreSupportedType(superClass);
    }
}

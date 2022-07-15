package me.danwi.sqlex.core.apt;

import me.danwi.sqlex.common.ParameterTypes;
import me.danwi.sqlex.core.annotation.repository.SqlExConverter;
import me.danwi.sqlex.core.type.ParameterConverter;

import javax.annotation.processing.ProcessingEnvironment;
import javax.lang.model.element.*;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.MirroredTypeException;
import javax.lang.model.type.TypeMirror;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import java.util.*;
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
        //获取其实现的ParameterConverter接口
        List<DeclaredType> declaredTypes = element.getInterfaces().stream()
                .map(it -> {
                    if (getQualifiedName(it).contentEquals(ParameterConverter.class.getName()) && it instanceof DeclaredType) {
                        return (DeclaredType) it;
                    }
                    return null;
                })
                .filter(Objects::nonNull)
                .collect(Collectors.toList());

        if (declaredTypes.size() == 0)
            throw new Exception("未实现参数类型转换器接口");
        DeclaredType converterInterface = declaredTypes.get(0);
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
        //获取转换器的来源类型和目的类型
        List<? extends TypeMirror> typeArguments = converterInterface.getTypeArguments();
        if (typeArguments.size() != 2)
            throw new Exception("转换器接口拥有错误的泛型参数个数");
        //获取其返回类型
        TypeMirror toType = typeArguments.get(1);
        //判断其是否是预支持类型
        if (!isSupportedType(types.asElement(toType), Collections.emptyList())) {
            throw new Exception("参数类型转换器的目标类型 " + getQualifiedName(toType) + " 不是预支持的数据类型");
        }
        //获取其来源类型
        TypeMirror typeMirror = typeArguments.get(0);
        if (typeMirror == null)
            throw new Exception("无法确定参数类型转换器的目标类型");
        return typeMirror;
    }


    //判断这个类型是否为支持的参数类型类型
    public boolean isSupportedType(Element element, List<String> additional) {
        String qualifiedName = getQualifiedName(element);
        //是否在预支持列表中
        if (Arrays.asList(ParameterTypes.PreSupportedTypes).contains(qualifiedName))
            return true;
        //是否在额外支持的列表中
        if (additional.contains(qualifiedName))
            return true;
        //获取父类,来判断
        Element superClass = getSuperClass(element);
        if (superClass == null)
            return false;
        return isSupportedType(superClass, additional);
    }
}

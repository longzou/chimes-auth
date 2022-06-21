
use std::iter::Map;
use serde::de::DeserializeOwned;

pub trait ChimesResource<T> 
  where
      T: Sized
  {
      /**
       * 获取资源的编码
       */
      fn get_code(&self) -> String;
  
      /**
       * 加载所有的资源
       */
      fn load_all_resources(&self) -> Map<String, T>;
  }
  
  pub trait ChimesAuthUser<T: Sized + DeserializeOwned> {
  
      /**
       * Chimes用户名
       */
      fn get_user_name(&self) -> String;
  
      /**
       * Chimes验证信息，如密码
       */
      fn get_creditial(&self) -> String;
  
      /**
       * 转换到T，这个用户的真实信息
       */
      fn to_detail(&self) -> &T;
  }
  
  pub trait ChimesAuthService<T>
  where
      T: Sized + ChimesAuthUser<T> + DeserializeOwned
  {
      /**
       * 检查用户是否能够通过指定的URL
       * 根据系统的配置来确定是否能够通过这个URL请求
       * 1. ust为None值
       *  此时根据req_method和url_pattern查询到该url为bypass=anonymous的模式，则返回true
       *  此时根据req_method和url_pattern查询到该url为bypass=user的模式，则返回false
       *  此时根据req_method和url_pattern查询到该url为bypass=permit的模式，则返回false
       * 2. ust为Some值
       *  此时根据req_method和url_pattern查询到该url为bypass=anonymous的模式，则返回true
       *  此时根据req_method和url_pattern查询到该url为bypass=user的模式，则返回true
       *  此时根据req_method和url_pattern查询到该url为bypass=permit的模式，则:
       *      a. 用户拥有可以访问该权限的角色：返回true
       *      b. 用户拥有可以访问该权限的资源: 返回true
       *      c. 用户不满足a和b，则返回false
       */
      fn permit(&self, ust: &Option<T>, req_method: &String, url_pattern: &String) -> bool;
  
  
      /**
       * 检查Authentication信息是否为有效的用户信息
       * 可以做如下处理
       * 1. token是一个唯一的字符串信息，通过以该token为key，从存储（数据库或Redis或内存）中查询到该Key所代表的用户信息；
       * 2. token是一个JWT Token，则解析该Token以获得登录的用户名等信息
       * 3. 再通过用户信息去查询用户详细信息
       * 4. 返回None表示该token已失效，返回Some表示该Token有效，且可以找到对应的帐户信息
       */
      fn authenticate(&self, token: &String) -> Option<T>;
  
}


# chimes-auth
这是一个使用actix-web的Middleware方式实现的认证与授权框架。

### 介绍

chimes-auth将这个过程分为两个部分，一个是Middleware部分，实现对service的拦截处理；另一个部分是一个RBAC的抽象模型。
chimes-auth的中间件名称为ChimesAuthorization，可以使用该结构来创建chimes-auth的中间件。使用方式为：
```
ChimesAuthorization::new(auth_service)
```
然后在Actix-Web的App中进行wrap注册。
```
App::new()
     .wrap(ChimesAuthorization::new(cuas.clone())
                        .header_key(&"Authentication".to_string())
                        .allow(&"/api/v1/login".to_string())
                        .allow(&"/api/v1/info".to_string())
     )
```
ChimesAuthorization提供了header_key，以及session_key（使用session feature时）两个可变参数，这两个参数表示从哪里获取Token；allow可以设置直接ByPass的URL，当访问到这些URL时将不会进行验证，而是直接执行后续的服务。

当使用header_key是，ChimesAuthorization中间件会从HTTP请求头中取得对应的值作为Authorization Token，并将其交由ChimesAuthService来验证，其将验证该Token是否为一个有效的Token，且该Token所对应的用户信息是不是有效的用户等。
如果打开了Session的Feature，则会根据session_key来取被保存在Session中的用户信息。

ChimesAuthorization在处理用户请求时，通常会有以下几种情况：

1. user为None值
此时根据req_method和url_pattern查询到该url为bypass=anonymous的模式，则返回Some(Default)
此时根据req_method和url_pattern查询到该url为bypass=user的模式，则返回None，此时应该是需要进行登录处理
此时根据req_method和url_pattern查询到该url为bypass=permit的模式，则返回None，则返回None，此时应该是需要进行登录处理
2. user为Some值
此时根据req_method和url_pattern查询到该url为bypass=anonymous的模式，则返回true
此时根据req_method和url_pattern查询到该url为bypass=user的模式，则返回true
此时根据req_method和url_pattern查询到该url为bypass=permit的模式，则:
   a. 用户拥有可以访问该权限的角色：返回Some(T)
   b. 用户拥有可以访问该权限的资源: 返回Some(T)
   c. 用户不满足a和b，则返回None

### chimes-auth的RBAC实践
在actix-web中，我们将一个请求的URL叫做一个资源（ChimesResource）。所以在chimes-auth的最小的访问单元是ChimesResource。
ChimesResource {
	method: String,
	url_pattern: String,
	by_pass: enum, // 可取值为anonymous（匿名可访问）, user（登录用户可访问）, permit（授权用户可访问）
}

接下就是WHO的问题，在chimes-auth中我们使用ChimesAuthUser的特征（trait）来表示，其实在ChimesAuthUser中，我们非常关心的用户的名称（user_name），它必须是唯一的，我们通过它可以查询到用户信息。

最终的问题是，怎么管理用户可以访问的资源。一种方式，就是我们需要建立ChimesAuthUser与ChimesResource中的对应关系，这是一个多对多的关系。通常为了更好的管理这些关系，还会建立Role体系（角色），也就是所谓的RBAC体系了。

当然，在chimes-auth中，并不需要这么复杂，那些RBAC都是管理方面的事情，chimes-auth不需要知道用户有什么角色，哪些角色可以访问哪些资源的问题。在chimes-auth中，整个体系只是回答了一个问题：当前用户可以访问当前请求吗？而这个问题，最终也是要交由项目的开发者来回答的。
所以，在chimes-auth中，需要实现特征ChimesAuthService：
1. authenticate 从当前请求中得到用户信息；
2. permit 判断当前请求的用户是否能够访问该请求；

### 例子
ChimesAuthUser特征的实现，以及ChimesAuthService特征的实现，如下：
```
#[derive(Clone, Default, Deserialize)]
pub struct SystemUser {
    user_name: String,
    password: String,
}

impl ChimesAuthUser<SystemUser> for SystemUser
{

    fn get_user_name(&self) -> String {
        self.user_name.clone()
    }

    fn get_creditial(&self) -> String {
        self.password.clone()
    }

    fn to_detail(&self) -> &SystemUser {
        self
    }
}

#[derive(Clone)]
pub struct ChimesUserAuthService<SystemUser> {
    #[allow(unused)]
    system_user: Option<SystemUser>
}

impl ChimesAuthService<SystemUser> for ChimesUserAuthService<SystemUser> {

    type Future = Pin<Box<dyn Future<Output=Option<SystemUser>>>>;

    fn permit(&self, ust: &Option<SystemUser>, req_method: &String, url_pattern: &String) -> Self::Future {
        let up = url_pattern.clone();
        Box::pin(async move {
            if up == "/" {
                return Some(SystemUser::default())
            } else {
                return None
            }
        })
    }

    fn authenticate(&self, token: &String) -> Self::Future {
        let rb = get_rbatis();
        Box::pin(async move {
            match MorinkhuurUser::from_id(rb, &1i64).await {
                Ok(r) => {
                    match r {
                        Some(u) => {
                            Some(SystemUser {
                                    user_name: u.username.unwrap(),
                                    password: u.api_password.unwrap(),
                                })
                        }
                        None => {
                            None
                        }
                    }
                }
                Err(_) => {
                    None
                }
            }
        })
    }
}

```
ChimesAuthorization在actix-web中的注册的例子：
```
async fn start_web_server(webconf: &WebServerConfig) -> std::io::Result<()> {
    // 设置服务器运行ip和端口信息
    let ip = format!("{}:{}", "0.0.0.0", webconf.port.clone());
    log::info!("App is listening on {}.", ip.clone());
    // 启动一个web服务
    let cuas = ChimesUserAuthService { system_user: None };
    
    
    HttpServer::new(move || {
        App::new()
            .wrap(ChimesAuthorization::new(cuas.clone())
                        .header_key(&"Authentication".to_string())
                        .allow(&"/api/v1/login".to_string())
                        .allow(&"/api/v1/info".to_string())
            )
            .service(index_handler)
            .service(crate::handler::query_user_paged)
            .service(crate::handler::query_user_query)
     })
    .bind(ip)?
    .run()
    .await
}
```





# 联系方式/捐赠,或 [rbatis-generator](https://github.com/longzou/rbatis-generator) 点star

> 捐赠

<img style="width: 400px;height: 545px;" width="400" height="545" src="https://gitee.com/poethxp/rbatis-generator/raw/master/wx_account.jpg" alt="enjoylost" />


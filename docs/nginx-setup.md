## Nginx Configuration for Server

### 1. Install nginx
[From Here](https://www.nginx.com/resources/wiki/start/topics/tutorials/install/)

### 2. Generate SSL Certificates
Make a ssl_cert directory in the home directory. Then run this command inside that folder.

```
openssl req -x509 -newkey rsa:2048 -keyout privkey.pem -out cert.pem -days 365 -nodes 
```

### 3. Nginx Configuration
Create a new file in /etc/nginx/sites-enabled/ directory. And paste this configuration after modifying the domanin.
```
	server {
	
	        server_name subdomain;
	        root /var/www/html;
	        index index.html;
	
	        location / {
	          try_files $uri $uri/ =404;
	
	          proxy_pass http://localhost:9944;
	          proxy_set_header X-Real-IP $remote_addr;
	          proxy_set_header Host $host;
	          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
	
	          proxy_http_version 1.1;
	          proxy_set_header Upgrade $http_upgrade;
	          proxy_set_header Connection "upgrade";
	        }
	
	        location /rpc {
	          try_files $uri $uri/ =404;
	
	          proxy_pass http://localhost:9934;
	          proxy_set_header X-Real-IP $remote_addr;
	          proxy_set_header Host $host;
	          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
	
	          proxy_http_version 1.1;
	          proxy_set_header Upgrade $http_upgrade;
	          proxy_set_header Connection "upgrade";
	        }
	
	        listen [::]:443 ssl ipv6only=on;
	        listen 443 ssl;
	        ssl_certificate /home/ubuntu/ssl_cert/cert.pem;
	        ssl_certificate_key /home/ubuntu/ssl_cert/privkey.pem;
	
	        ssl_session_cache shared:cache_nginx_SSL:1m;
	        ssl_session_timeout 1440m;
	
	        ssl_protocols TLSv1 TLSv1.1 TLSv1.2;
	        ssl_prefer_server_ciphers off;
	
	    }
```

### 4. Start nginx service
You might need to delete default file in nginx config directory.

```
    sudo service nginx start
```

```
    sudo service nginx restart
```

### 5. Domain Config
Add CNAME entry in domain management for the domain mentioned in the nginx config
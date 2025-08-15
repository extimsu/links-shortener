# ---- Build Stage ----
FROM node:20-alpine as build
WORKDIR /app
COPY . .
RUN npm ci && npm run build

# ---- Production Stage ----
FROM nginx:alpine
LABEL maintainer="Your Name <your@email.com>"
LABEL org.opencontainers.image.source="https://github.com/your/repo"
COPY --from=build /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY .htpasswd /etc/nginx/.htpasswd
EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]

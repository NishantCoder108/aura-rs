### Important Stuff:
#### How to deploy on Hugging Face Spaces:



### Steps:
- Create a new space on Hugging Face
- Add ssh key to the space from your local machine of your github account
- Add the following files:
  - .dockerignore
  - Dockerfile
  - .env.example
  - README.md
- Commit and push the changes to the  github
- Add set url of the hugging space inside project  using ssh key, that is already setup
- Now, push the code on hugging face space
  ```bash
    git push hf master:main
  ```

- Now, we can check the deployment on the hugging face space
- Space url looks like this: https://huggingface.co/spaces/organizationName/projectName , for setting stuff.
- For api endpoint: https://usernameOfSpace-projectName.hf.space

- We can test this api using curl or postman or we can add to client side as well

- For every time, we change the code, first we commit and push to github, then push to hugging face space using the following command:
  ```bash
    git push hf master:main
  ```